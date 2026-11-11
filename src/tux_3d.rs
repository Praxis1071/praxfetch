//! Inline, mouse-interactive 3D Tux for Ratty terminals.
//!
//! This intentionally does not create a Ratatui terminal, alternate screen, or
//! full-screen interface. The 3D object occupies the same cells as the classic
//! seven-line ASCII logo; Praxfetch's system-information columns stay untouched.

use std::{
    io::{self, IsTerminal, Write},
    time::Duration,
};

use crossterm::{
    cursor,
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseButton, MouseEventKind},
    execute,
    terminal,
};
use ratatui::layout::Rect;
use ratatui_ratty::{ObjectFormat, RattyGraphic, RattyGraphicSettings};

use crate::output::OutputLayout;

const MODEL_ID_BASE: u32 = 71_071;
const EVENT_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Print the usual Praxfetch layout and replace only its ASCII logo with Tux.
pub fn run(output: &OutputLayout) -> io::Result<()> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(io::Error::new(
            io::ErrorKind::NotConnected,
            "interactive 3D mode requires a terminal; run it inside Ratty",
        ));
    }

    // RGP uses absolute terminal-cell coordinates. Ask the terminal where this
    // command's output begins so the model is placed correctly below the prompt.
    let origin = cursor::position()?;
    let mut terminal_mode = TerminalMode::enter()?;

    let result = run_inline(output, origin);
    let restore_result = terminal_mode.restore();

    result.and(restore_result)
}

fn run_inline(output: &OutputLayout, origin: (u16, u16)) -> io::Result<()> {
    let mut model = TuxModel::new()?;
    let area = Rect::new(origin.0, origin.1, output.logo_width(), output.logo_height());

    let interaction = (|| {
        output.print_with_3d_logo_slot()?;
        model.place(area)?;
        interact(&mut model, area)
    })();

    // Delete only the object IDs created by this Praxfetch process, then put
    // the ordinary ASCII Tux back into the exact same output column.
    let object_cleanup = model.clear();
    let text_restore = output.restore_ascii_logo(origin);

    interaction.and(object_cleanup).and(text_restore)
}

struct TerminalMode {
    raw: bool,
    mouse: bool,
}

impl TerminalMode {
    fn enter() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        let mut mode = Self {
            raw: true,
            mouse: false,
        };

        if let Err(error) = execute!(io::stdout(), EnableMouseCapture) {
            let _ = mode.restore();
            return Err(error);
        }
        mode.mouse = true;
        Ok(mode)
    }

    fn restore(&mut self) -> io::Result<()> {
        let mut first_error = None;

        if self.mouse {
            if let Err(error) = execute!(io::stdout(), DisableMouseCapture) {
                first_error = Some(error);
            }
            self.mouse = false;
        }

        if self.raw {
            if let Err(error) = terminal::disable_raw_mode() {
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
            self.raw = false;
        }

        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

impl Drop for TerminalMode {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

fn interact(model: &mut TuxModel, area: Rect) -> io::Result<()> {
    let mut drag_position = None;

    loop {
        if !event::poll(EVENT_POLL_INTERVAL)? {
            continue;
        }

        match event::read()? {
            Event::Key(key) if key.code == KeyCode::Esc || key.code == KeyCode::Char('q') || key.code == KeyCode::Char('Q') => {
                return Ok(());
            }
            Event::Mouse(mouse) => {
                let x = mouse.column;
                let y = mouse.row;
                let inside = x >= area.x && x < area.right() && y >= area.y && y < area.bottom();

                match mouse.kind {
                    MouseEventKind::Down(MouseButton::Left) if inside => {
                        drag_position = Some((x, y));
                    }
                    MouseEventKind::Drag(MouseButton::Left) => {
                        if let Some((previous_x, previous_y)) = drag_position {
                            let dx = i32::from(x) - i32::from(previous_x);
                            let dy = i32::from(y) - i32::from(previous_y);
                            model.rotate(dx as f32 * 2.0, dy as f32 * 2.0)?;
                        }
                        drag_position = if inside { Some((x, y)) } else { None };
                    }
                    MouseEventKind::Up(MouseButton::Left) => {
                        drag_position = None;
                    }
                    MouseEventKind::ScrollUp if inside => model.zoom(0.08)?,
                    MouseEventKind::ScrollDown if inside => model.zoom(-0.08)?,
                    _ => {}
                }
            }
            _ => {}
        }
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
            let mesh = ellipsoid_obj(center, radii, 20, 12);

            if let Err(error) = graphic.register_payload_with_name(mesh.as_bytes(), Some(name)) {
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

    fn place(&self, area: Rect) -> io::Result<()> {
        let mut stdout = io::stdout().lock();
        for part in &self.parts {
            stdout.write_all(part.place_sequence(area).as_bytes())?;
        }
        stdout.flush()
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
        let obj = ellipsoid_obj([0.2, -0.3, 0.4], [0.5, 0.6, 0.7], 20, 12);
        for line in obj.lines().filter(|line| line.starts_with("v ")) {
            let values = line
                .split_whitespace()
                .skip(1)
                .map(|value| value.parse::<f32>().unwrap());
            assert!(values.into_iter().all(f32::is_finite));
        }
    }
}
