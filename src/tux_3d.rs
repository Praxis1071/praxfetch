//! Interactive 3D Tux view powered by Orhun Parmaksız's Ratty Graphics Protocol.

use std::{io::{self, Write}, time::Duration};

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseButton,
        MouseEventKind,
    },
    execute,
};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};
use ratatui_ratty::{ObjectFormat, RattyGraphic, RattyGraphicSettings};

use crate::info::{self, SystemInfo};

const MODEL_ID_BASE: u32 = 71_071;
const FRAME_INTERVAL: Duration = Duration::from_millis(33);

/// Start the optional Ratty-backed interactive 3D Tux view.
///
/// This mode requires the Ratty terminal emulator. The regular CLI output
/// remains independent from Ratty and does not enter an interactive event loop.
pub fn run() -> io::Result<()> {
    let mut terminal = ratatui::init();

    let result = (|| {
        execute!(io::stdout(), EnableMouseCapture)?;
        if let Err(error) = activate_tux_camera() {
            let _ = activate_default_camera_slot();
            let _ = execute!(io::stdout(), DisableMouseCapture);
            return Err(error);
        }
        let result = run_app(&mut terminal);
        let camera_cleanup = activate_default_camera_slot();
        let mouse_cleanup = execute!(io::stdout(), DisableMouseCapture);
        result.and(camera_cleanup).and(mouse_cleanup)
    })();

    ratatui::restore();
    result
}


/// Use a dedicated Ratty camera slot so the user's default slot is not overwritten.
fn activate_tux_camera() -> io::Result<()> {
    io::stdout().write_all(
        b"\x1b_ratty;g;c;id=9;set=1;type=Ortho;scale=0.95;rx=8;ry=18;rz=2\x1b\\",
    )?;
    io::stdout().flush()
}

/// Return to camera slot zero without changing its saved projection or transform.
fn activate_default_camera_slot() -> io::Result<()> {
    io::stdout().write_all(b"\x1b_ratty;g;c;id=0;set=1\x1b\\")?;
    io::stdout().flush()
}

fn run_app(terminal: &mut DefaultTerminal) -> io::Result<()> {
    let mut app = TuxApp::new()?;
    let result = app.run(terminal);
    let cleanup = app.clear();
    result.and(cleanup)
}

struct TuxApp {
    system: SystemInfo,
    model: TuxModel,
    viewport: Rect,
    drag_position: Option<(u16, u16)>,
}

impl TuxApp {
    fn new() -> io::Result<Self> {
        Ok(Self {
            system: info::collect(),
            model: TuxModel::new()?,
            viewport: Rect::default(),
            drag_position: None,
        })
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        loop {
            terminal.draw(|frame| self.draw(frame))?;

            if !event::poll(FRAME_INTERVAL)? {
                continue;
            }

            match event::read()? {
                Event::Key(key)
                    if key.code == KeyCode::Esc
                        || key.code == KeyCode::Char('q')
                        || key.code == KeyCode::Char('Q') =>
                {
                    return Ok(());
                }
                Event::Mouse(mouse) => self.handle_mouse(mouse.kind, mouse.column, mouse.row)?,
                Event::Resize(_, _) => self.drag_position = None,
                _ => {}
            }
        }
    }

    fn draw(&mut self, frame: &mut Frame<'_>) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(frame.area());
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
            .split(rows[0]);

        let tux_block = Block::default()
            .title(" TUX // 3D ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        self.viewport = tux_block.inner(chunks[0]);
        frame.render_widget(tux_block, chunks[0]);

        for part in &self.model.parts {
            part.render(self.viewport, frame.buffer_mut());
        }

        let system_block = Block::default()
            .title(" SYSTEM ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue));
        let host = if !self.system.host.product.is_empty() {
            self.system.host.product.clone()
        } else if !self.system.host.vendor.is_empty() {
            self.system.host.vendor.clone()
        } else {
            "unknown".to_owned()
        };
        let packages = self
            .system
            .packages
            .count
            .map(|count| count.to_string())
            .unwrap_or_else(|| "unknown".to_owned());
        let fields = [
            ("", format!("{}@{}", self.system.user, self.system.hostname)),
            ("os", info::os_label(&self.system.os)),
            ("host", host),
            ("kernel", self.system.kernel.clone()),
            (
                "uptime",
                self.system
                    .uptime
                    .map(info::uptime)
                    .unwrap_or_else(|| "unknown".to_owned()),
            ),
            ("pkgs", packages),
            ("memory", info::memory_compact(&self.system.memory)),
        ];
        let label_width = fields
            .iter()
            .map(|(label, _)| label.chars().count())
            .max()
            .unwrap_or(0);
        let lines = fields
            .iter()
            .map(|(label, value)| {
                if label.is_empty() {
                    Line::from(Span::styled(
                        value.clone(),
                        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                    ))
                } else {
                    Line::from(vec![
                        Span::styled(
                            format!("{label:<label_width$} "),
                            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(value.clone()),
                    ])
                }
            })
            .collect::<Vec<_>>();
        let system = Paragraph::new(lines)
            .block(system_block)
            .style(Style::default().fg(Color::White));
        frame.render_widget(system, chunks[1]);

        let footer = Paragraph::new(Line::from(vec![
            Span::styled(
                "Drag",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" to rotate  "),
            Span::styled(
                "Scroll",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" to zoom  "),
            Span::styled(
                "Q / Esc",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" to exit"),
        ]));
        frame.render_widget(footer, rows[1]);
    }

    fn handle_mouse(
        &mut self,
        kind: MouseEventKind,
        column: u16,
        row: u16,
    ) -> io::Result<()> {
        let inside = column >= self.viewport.x
            && column < self.viewport.right()
            && row >= self.viewport.y
            && row < self.viewport.bottom();

        match kind {
            MouseEventKind::Down(MouseButton::Left) if inside => {
                self.drag_position = Some((column, row));
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if let Some((previous_x, previous_y)) = self.drag_position {
                    let dx = i32::from(column) - i32::from(previous_x);
                    let dy = i32::from(row) - i32::from(previous_y);
                    self.model.rotate(dx as f32 * 2.0, dy as f32 * 2.0)?;
                }
                self.drag_position = Some((column, row));
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.drag_position = None;
            }
            MouseEventKind::ScrollUp if inside => self.model.zoom(0.08)?,
            MouseEventKind::ScrollDown if inside => self.model.zoom(-0.08)?,
            _ => {}
        }

        Ok(())
    }

    fn clear(&self) -> io::Result<()> {
        self.model.clear()
    }
}

struct TuxModel {
    parts: Vec<RattyGraphic<'static>>,
    yaw: f32,
    pitch: f32,
    scale: f32,
}

impl TuxModel {
    fn new() -> io::Result<Self> {
        let definitions = [
            ("body", [20, 27, 39], [0.0, -0.05, 0.0], [0.56, 0.78, 0.40]),
            ("head", [20, 27, 39], [0.0, 0.68, 0.0], [0.43, 0.43, 0.36]),
            ("left-wing", [20, 27, 39], [-0.48, -0.12, 0.02], [0.16, 0.47, 0.23]),
            ("right-wing", [20, 27, 39], [0.48, -0.12, 0.02], [0.16, 0.47, 0.23]),
            ("belly", [238, 242, 245], [0.0, -0.12, 0.34], [0.39, 0.60, 0.12]),
            ("face", [238, 242, 245], [0.0, 0.70, 0.31], [0.27, 0.23, 0.09]),
            ("left-eye", [245, 248, 250], [-0.14, 0.80, 0.39], [0.075, 0.09, 0.055]),
            ("right-eye", [245, 248, 250], [0.14, 0.80, 0.39], [0.075, 0.09, 0.055]),
            ("left-pupil", [15, 19, 25], [-0.14, 0.80, 0.44], [0.034, 0.045, 0.025]),
            ("right-pupil", [15, 19, 25], [0.14, 0.80, 0.44], [0.034, 0.045, 0.025]),
            ("beak", [245, 145, 28], [0.0, 0.53, 0.47], [0.14, 0.085, 0.13]),
            ("left-foot", [235, 125, 24], [-0.25, -0.80, 0.18], [0.24, 0.10, 0.19]),
            ("right-foot", [235, 125, 24], [0.25, -0.80, 0.18], [0.24, 0.10, 0.19]),
        ];

        let mut parts: Vec<RattyGraphic<'static>> = Vec::with_capacity(definitions.len());
        for (index, (name, color, center, radii)) in definitions.into_iter().enumerate() {
            let id = MODEL_ID_BASE
                .wrapping_add(std::process::id().wrapping_mul(16))
                .wrapping_add(index as u32);
            let settings = RattyGraphicSettings::new(format!("praxfetch-tux-{name}.obj"))
                .id(id)
                .format(ObjectFormat::Obj)
                .normalize(false)
                .animate(false)
                .scale(0.95)
                .color(color)
                .brightness(1.0);
            let graphic = RattyGraphic::new(settings);
            let mesh = ellipsoid_obj(center, radii, 16, 10);
            if let Err(error) =
                graphic.register_payload_with_name(mesh.as_bytes(), Some(name))
            {
                let _ = graphic.clear();
                for registered in &parts {
                    let _ = registered.clear();
                }
                return Err(error);
            }
            parts.push(graphic);
        }

        Ok(Self {
            parts,
            yaw: -18.0,
            pitch: 5.0,
            scale: 0.95,
        })
    }

    fn rotate(&mut self, yaw_delta: f32, pitch_delta: f32) -> io::Result<()> {
        self.yaw = (self.yaw + yaw_delta).rem_euclid(360.0);
        self.pitch = (self.pitch + pitch_delta).clamp(-80.0, 80.0);
        self.update()
    }

    fn zoom(&mut self, delta: f32) -> io::Result<()> {
        self.scale = (self.scale + delta).clamp(0.35, 2.2);
        self.update()
    }

    fn update(&mut self) -> io::Result<()> {
        for part in &mut self.parts {
            let settings = part.settings_mut();
            settings.rotation = [self.pitch, self.yaw, 0.0];
            settings.scale = self.scale;
            part.update()?;
        }
        Ok(())
    }

    fn clear(&self) -> io::Result<()> {
        let mut first_error = None;
        for part in &self.parts {
            if let Err(error) = part.clear() {
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
        }

        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

fn ellipsoid_obj(
    center: [f32; 3],
    radii: [f32; 3],
    segments: usize,
    rings: usize,
) -> String {
    let mut obj = String::with_capacity((segments * rings * 48) + 128);
    obj.push_str("o tux_part\n");

    for ring in 0..=rings {
        let phi = std::f32::consts::PI * ring as f32 / rings as f32;
        let y = phi.cos();
        let radial = phi.sin();
        for segment in 0..segments {
            let theta = std::f32::consts::TAU * segment as f32 / segments as f32;
            let x = radial * theta.cos();
            let z = radial * theta.sin();
            obj.push_str(&format!(
                "v {:.5} {:.5} {:.5}\n",
                center[0] + x * radii[0],
                center[1] + y * radii[1],
                center[2] + z * radii[2]
            ));
            let nx = x / radii[0];
            let ny = y / radii[1];
            let nz = z / radii[2];
            let normal_length = (nx * nx + ny * ny + nz * nz)
                .sqrt()
                .max(f32::EPSILON);
            obj.push_str(&format!(
                "vn {:.5} {:.5} {:.5}\n",
                nx / normal_length,
                ny / normal_length,
                nz / normal_length
            ));
        }
    }

    for ring in 0..rings {
        for segment in 0..segments {
            let next = (segment + 1) % segments;
            let a = ring * segments + segment + 1;
            let b = ring * segments + next + 1;
            let c = (ring + 1) * segments + next + 1;
            let d = (ring + 1) * segments + segment + 1;
            obj.push_str(&format!(
                "f {a}//{a} {b}//{b} {c}//{c}\nf {a}//{a} {c}//{c} {d}//{d}\n"
            ));
        }
    }

    obj
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_tux_parts_are_valid_obj_meshes() {
        let obj = ellipsoid_obj([0.0, 0.0, 0.0], [1.0, 2.0, 1.0], 12, 8);
        assert!(obj.starts_with("o tux_part\n"));
        assert_eq!(obj.lines().filter(|line| line.starts_with("v ")).count(), 108);
        assert_eq!(obj.lines().filter(|line| line.starts_with("vn ")).count(), 108);
        assert_eq!(obj.lines().filter(|line| line.starts_with("f ")).count(), 192);
        assert!(obj.lines().all(|line| !line.contains("NaN")));
    }

    #[test]
    fn generated_tux_mesh_uses_only_finite_coordinates() {
        let obj = ellipsoid_obj([0.2, -0.3, 0.4], [0.5, 0.6, 0.7], 16, 10);
        for line in obj.lines().filter(|line| line.starts_with("v ")) {
            let values = line
                .split_whitespace()
                .skip(1)
                .map(|value| value.parse::<f32>().unwrap());
            assert!(values.into_iter().all(f32::is_finite));
        }
    }
}
