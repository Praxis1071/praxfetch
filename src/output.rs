use std::io::{self, Write};

use colored::Colorize;

use crate::{ascii, info::{self, SystemInfo}};

/// The classic seven-line Praxfetch presentation, shared by normal and 3D modes.
pub struct OutputLayout {
    logo: Vec<String>,
    colored_logo_columns: Vec<String>,
    logo_column_width: usize,
    right_columns: Vec<String>,
}

impl OutputLayout {
    pub fn new(system: &SystemInfo) -> Self {
        let logo = ascii::logo(&system.os);
        let host = if !system.host.product.is_empty() {
            system.host.product.clone()
        } else if !system.host.vendor.is_empty() {
            system.host.vendor.clone()
        } else {
            "unknown".to_owned()
        };

        let packages = system
            .packages
            .count
            .map(|count| count.to_string())
            .unwrap_or_else(|| "unknown".to_owned());

        let fields = [
            ("", format!("{}@{}", system.user, system.hostname)),
            ("os", info::os_label(&system.os)),
            ("host", host),
            ("kernel", system.kernel.clone()),
            (
                "uptime",
                system
                    .uptime
                    .map(info::uptime)
                    .unwrap_or_else(|| "unknown".to_owned()),
            ),
            ("pkgs", packages),
            ("memory", info::memory_compact(&system.memory)),
        ];

        let label_width = fields
            .iter()
            .map(|(key, _)| key.chars().count())
            .max()
            .unwrap_or(0);

        let logo_column_width = ascii::logo_width(&logo) + 4;
        let colored_logo_columns: Vec<String> = logo
            .iter()
            .map(|line| {
                let padded = ascii::pad_logo_line(line, logo_column_width);
                ascii::colorize_logo_line(&padded, &system.os)
            })
            .collect();

        let right_columns: Vec<String> = fields
            .iter()
            .map(|(key, value)| {
                if key.is_empty() {
                    value.green().bold().to_string()
                } else {
                    let key = format!("{key:<label_width$}");
                    format!("{} {}", key.cyan().bold(), value)
                }
            })
            .collect();

        Self {
            logo_column_width,
            logo,
            colored_logo_columns,
            right_columns,
        }
    }

    pub fn logo_width(&self) -> u16 {
        self.logo_column_width.min(usize::from(u16::MAX)) as u16
    }

    pub fn logo_height(&self) -> u16 {
        self.logo.len().min(usize::from(u16::MAX)) as u16
    }

    /// Print the original Praxfetch output without changing its layout.
    pub fn print_classic(&self) -> io::Result<()> {
        let mut stdout = io::stdout().lock();
        for row in 0..self.row_count() {
            let left = self.logo_column(row);
            let right = self.right_columns.get(row).map(String::as_str).unwrap_or("");
            writeln!(stdout, "{left}{right}")?;
        }
        stdout.flush()
    }

    /// Print the same output, reserving only the original logo column for RGP.
    ///
    /// Raw terminal mode is active in this path, so CRLF is written explicitly.
    pub fn print_with_3d_logo_slot(&self) -> io::Result<()> {
        let mut stdout = io::stdout().lock();
        let blank = " ".repeat(self.logo_column_width);
        for row in 0..self.row_count() {
            let right = self.right_columns.get(row).map(String::as_str).unwrap_or("");
            write!(stdout, "{blank}{right}\r\n")?;
        }
        stdout.flush()
    }

    /// Replace the inline 3D object with the ordinary ASCII logo on exit.
    pub fn restore_ascii_logo(&self, origin: (u16, u16)) -> io::Result<()> {
        use crossterm::{cursor::MoveTo, execute};

        let mut stdout = io::stdout().lock();
        for row in 0..self.logo.len() {
            let y = origin.1.saturating_add(row.min(usize::from(u16::MAX)) as u16);
            execute!(stdout, MoveTo(origin.0, y))?;
            write!(stdout, "{}", self.logo_column(row))?;
        }

        let end_row = origin.1.saturating_add(self.row_count().min(usize::from(u16::MAX)) as u16);
        execute!(stdout, MoveTo(origin.0, end_row))?;
        stdout.flush()
    }

    fn row_count(&self) -> usize {
        self.logo.len().max(self.right_columns.len())
    }

    fn logo_column(&self, row: usize) -> String {
        self.colored_logo_columns
            .get(row)
            .cloned()
            .unwrap_or_else(|| " ".repeat(self.logo_column_width))
    }
}
