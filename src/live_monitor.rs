use crate::compact::CompactReport;
use std::env;
use std::fmt::Write as _;
use std::io::{self, Stdout, Write};
#[cfg(unix)]
use std::os::fd::AsRawFd;

const RESET: &str = "\x1b[0m";
const GREEN: &str = "\x1b[32m";
const GRAY: &str = "\x1b[90m";
const CYAN: &str = "\x1b[36m";
const YELLOW: &str = "\x1b[33m";

const CANVAS_WIDTH: usize = 125;
const CANVAS_HEIGHT: usize = 44;
const LEFT_PAD_CENTER_X: usize = 22;
const LEFT_STICK_CENTER_X: usize = 40;
const CENTER_X: usize = 62;
const RIGHT_STICK_CENTER_X: usize = 84;
const RIGHT_FACE_CENTER_X: usize = 102;
const STICK_PIXEL_DIAMETER: usize = 29;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayMode {
    Full,
    Raw,
    Compact,
    None,
}

impl DisplayMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Full => "graphic",
            Self::Raw => "raw",
            Self::Compact => "compact",
            Self::None => "none",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::Full => {
                "DS4 Live Monitor                                          Ctrl-C to exit"
            }
            Self::Raw => "DS4 Raw HID Monitor                                       Ctrl-C to exit",
            Self::Compact => {
                "DS4 Compact Monitor                                       Ctrl-C to exit"
            }
            Self::None => {
                "DS4 Output                                                Ctrl-C to exit"
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct MonitorFrame {
    pub sequence: u64,
    pub transport: &'static str,
    pub report_len: usize,
    pub device_name: String,
    pub vendor_id: u16,
    pub product_id: u16,
    pub interface_number: i32,
    pub raw_report: Vec<u8>,
    pub compact: CompactReport,
}

impl MonitorFrame {
    pub fn idle() -> Self {
        Self {
            sequence: 0,
            transport: "waiting",
            report_len: 0,
            device_name: String::from("unknown"),
            vendor_id: 0,
            product_id: 0,
            interface_number: 0,
            raw_report: Vec::new(),
            compact: [0, 0, 128, 128, 128, 128, 0, 0],
        }
    }
}

pub struct MonitorUi {
    mode: DisplayMode,
    stdout: Stdout,
}

impl MonitorUi {
    pub fn new(mode: DisplayMode) -> io::Result<Self> {
        let mut stdout = io::stdout();
        write!(stdout, "\x1b[2J\x1b[H\x1b[?25l")?;
        stdout.flush()?;
        Ok(Self { mode, stdout })
    }

    pub fn render(&mut self, frame: &MonitorFrame, status: Option<&str>) -> io::Result<()> {
        let mut screen = String::new();

        writeln!(
            screen,
            "{}",
            pad_visible_line(self.mode.title(), CANVAS_WIDTH)
        )
        .expect("writing to String should not fail");
        writeln!(screen).expect("writing to String should not fail");

        match self.mode {
            DisplayMode::Full => {
                render_full_monitor(&mut screen, frame);
                write_common_monitor_lines(&mut screen, frame, status, self.mode);
                write_wrapped_colored_line(
                    &mut screen,
                    "Compact: ",
                    &format_report_hex(&frame.compact),
                    GRAY,
                );
                write_wrapped_colored_line(
                    &mut screen,
                    "HID: ",
                    &format_report_hex(&frame.raw_report),
                    GRAY,
                );
            }
            DisplayMode::Raw => {
                write_common_monitor_lines(&mut screen, frame, status, self.mode);
                render_raw_monitor(&mut screen, frame);
            }
            DisplayMode::Compact => {
                write_common_monitor_lines(&mut screen, frame, status, self.mode);
                render_compact_monitor(&mut screen, frame);
            }
            DisplayMode::None => {}
        }

        let fitted_screen = fit_screen_to_terminal(&screen, CANVAS_WIDTH);
        write!(self.stdout, "\x1b[H{fitted_screen}\x1b[J")?;
        self.stdout.flush()
    }
}

impl Drop for MonitorUi {
    fn drop(&mut self) {
        let _ = write!(self.stdout, "\x1b[2J\x1b[H\x1b[0m\x1b[?25h");
        let _ = self.stdout.flush();
    }
}

fn render_full_monitor(screen: &mut String, frame: &MonitorFrame) {
    let view = CompactView::new(frame.compact);
    let mut canvas = Canvas::new(CANVAS_WIDTH, CANVAS_HEIGHT);

    draw_bitmap_trigger_button(&mut canvas, LEFT_PAD_CENTER_X, 0, "L2", view.l2_raw());
    draw_bitmap_trigger_button(&mut canvas, RIGHT_FACE_CENTER_X, 0, "R2", view.r2_raw());
    draw_bitmap_shoulder_button(&mut canvas, LEFT_PAD_CENTER_X, 6, "L1", view.l1_pressed());
    draw_bitmap_shoulder_button(&mut canvas, RIGHT_FACE_CENTER_X, 6, "R1", view.r1_pressed());

    draw_dot_pattern(
        &mut canvas,
        LEFT_PAD_CENTER_X,
        13,
        dpad_up_bitmap(),
        view.up_pressed(),
    );
    draw_dot_pattern(
        &mut canvas,
        LEFT_PAD_CENTER_X - 6,
        16,
        dpad_left_bitmap(),
        view.left_pressed(),
    );
    draw_dot_pattern(
        &mut canvas,
        LEFT_PAD_CENTER_X + 6,
        16,
        dpad_right_bitmap(),
        view.right_pressed(),
    );
    draw_dot_pattern(
        &mut canvas,
        LEFT_PAD_CENTER_X,
        19,
        dpad_down_bitmap(),
        view.down_pressed(),
    );

    draw_dot_pattern(
        &mut canvas,
        RIGHT_FACE_CENTER_X,
        12,
        triangle_bitmap(),
        view.triangle_pressed(),
    );
    draw_dot_pattern(
        &mut canvas,
        RIGHT_FACE_CENTER_X - 9,
        16,
        square_bitmap(),
        view.square_pressed(),
    );
    draw_dot_pattern(
        &mut canvas,
        RIGHT_FACE_CENTER_X + 9,
        16,
        circle_bitmap(),
        view.circle_pressed(),
    );
    draw_dot_pattern(
        &mut canvas,
        RIGHT_FACE_CENTER_X,
        20,
        cross_bitmap(),
        view.cross_pressed(),
    );

    draw_bitmap_filled_button(&mut canvas, CENTER_X, 12, view.trackpad_pressed(), 54, 33);
    draw_bitmap_capsule_button(
        &mut canvas,
        CENTER_X - 18,
        12,
        "SHARE",
        view.share_pressed(),
    );
    draw_bitmap_capsule_button(
        &mut canvas,
        CENTER_X + 18,
        12,
        "OPTIONS",
        view.options_pressed(),
    );

    draw_bitmap_stick(
        &mut canvas,
        LEFT_STICK_CENTER_X,
        30,
        view.lx_raw(),
        view.ly_raw(),
        view.l3_pressed(),
        StickSide::Left,
    );
    draw_bitmap_stick(
        &mut canvas,
        RIGHT_STICK_CENTER_X,
        30,
        view.rx_raw(),
        view.ry_raw(),
        view.r3_pressed(),
        StickSide::Right,
    );

    draw_bitmap_circle_button(&mut canvas, CENTER_X, 29, "PS", view.ps_pressed());
    let stick_char_width = STICK_PIXEL_DIAMETER.div_ceil(2);
    put_centered_in_box(
        &mut canvas,
        LEFT_STICK_CENTER_X.saturating_sub(stick_char_width / 2),
        stick_char_width,
        36,
        "Left Stick",
        Color::Gray,
    );
    put_centered_in_box(
        &mut canvas,
        RIGHT_STICK_CENTER_X.saturating_sub(stick_char_width / 2),
        stick_char_width,
        36,
        "Right Stick",
        Color::Gray,
    );
    canvas.put_centered(
        LEFT_STICK_CENTER_X,
        37,
        &format!(
            "X:{:+4}% Y:{:+4}%",
            stick_percent_x(view.lx_raw()),
            stick_percent_y(view.ly_raw())
        ),
        Color::Gray,
    );
    canvas.put_centered(
        RIGHT_STICK_CENTER_X,
        37,
        &format!(
            "X:{:+4}% Y:{:+4}%",
            stick_percent_x(view.rx_raw()),
            stick_percent_y(view.ry_raw())
        ),
        Color::Gray,
    );

    screen.push_str(&canvas.render());
}

fn render_raw_monitor(screen: &mut String, frame: &MonitorFrame) {
    write_wrapped_colored_line(screen, "HID: ", &format_report_hex(&frame.raw_report), GRAY);
    writeln!(screen).expect("writing to String should not fail");
}

fn render_compact_monitor(screen: &mut String, frame: &MonitorFrame) {
    write_wrapped_colored_line(
        screen,
        "Compact: ",
        &format_report_hex(&frame.compact),
        GRAY,
    );
    writeln!(screen).expect("writing to String should not fail");
}

fn write_common_monitor_lines(
    screen: &mut String,
    frame: &MonitorFrame,
    status: Option<&str>,
    mode: DisplayMode,
) {
    if mode == DisplayMode::Full {
        write_colored_line(
            screen,
            &format!(
                "Seq: {:>6}   Transport: {:<10}   Bytes: {:>3}",
                frame.sequence, frame.transport, frame.report_len
            ),
            GRAY,
        );
    }
    match status {
        Some(message) => write_colored_line(screen, &format!("Status: {message}"), YELLOW),
        None => write_colored_line(screen, "Status: receiving", GRAY),
    }
    write_wrapped_colored_line(
        screen,
        "Device: ",
        &format!(
            "{}   VID:0x{:04X}   PID:0x{:04X}   IF:{}",
            frame.device_name, frame.vendor_id, frame.product_id, frame.interface_number
        ),
        GRAY,
    );
}

#[derive(Debug, Clone, Copy)]
struct CompactView {
    report: CompactReport,
}

impl CompactView {
    fn new(report: CompactReport) -> Self {
        Self { report }
    }

    fn button0(self, bit: u8) -> bool {
        (self.report[0] & (1u8 << bit)) != 0
    }

    fn button1(self, bit: u8) -> bool {
        (self.report[1] & (1u8 << bit)) != 0
    }

    fn analog(self, index: usize) -> u8 {
        self.report[index]
    }

    fn up_pressed(self) -> bool {
        self.button0(0)
    }

    fn right_pressed(self) -> bool {
        self.button0(1)
    }

    fn down_pressed(self) -> bool {
        self.button0(2)
    }

    fn left_pressed(self) -> bool {
        self.button0(3)
    }

    fn square_pressed(self) -> bool {
        self.button0(4)
    }

    fn cross_pressed(self) -> bool {
        self.button0(5)
    }

    fn circle_pressed(self) -> bool {
        self.button0(6)
    }

    fn triangle_pressed(self) -> bool {
        self.button0(7)
    }

    fn l1_pressed(self) -> bool {
        self.button1(0)
    }

    fn r1_pressed(self) -> bool {
        self.button1(1)
    }

    fn share_pressed(self) -> bool {
        self.button1(2)
    }

    fn options_pressed(self) -> bool {
        self.button1(3)
    }

    fn l3_pressed(self) -> bool {
        self.button1(4)
    }

    fn r3_pressed(self) -> bool {
        self.button1(5)
    }

    fn ps_pressed(self) -> bool {
        self.button1(6)
    }

    fn trackpad_pressed(self) -> bool {
        self.button1(7)
    }

    fn lx_raw(self) -> u8 {
        self.analog(2)
    }

    fn ly_raw(self) -> u8 {
        self.analog(3)
    }

    fn rx_raw(self) -> u8 {
        self.analog(4)
    }

    fn ry_raw(self) -> u8 {
        self.analog(5)
    }

    fn l2_raw(self) -> u8 {
        self.analog(6)
    }

    fn r2_raw(self) -> u8 {
        self.analog(7)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Color {
    Default,
    Gray,
    Green,
    Cyan,
}

impl Color {
    fn ansi(self) -> &'static str {
        match self {
            Self::Default => RESET,
            Self::Gray => GRAY,
            Self::Green => GREEN,
            Self::Cyan => CYAN,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Cell {
    ch: char,
    color: Color,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            color: Color::Default,
        }
    }
}

struct Canvas {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
}

impl Canvas {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![Cell::default(); width * height],
        }
    }

    fn put(&mut self, x: usize, y: usize, ch: char, color: Color) {
        if x >= self.width || y >= self.height {
            return;
        }

        self.cells[y * self.width + x] = Cell { ch, color };
    }

    fn put_text(&mut self, x: usize, y: usize, text: &str, color: Color) {
        for (offset, ch) in text.chars().enumerate() {
            self.put(x + offset, y, ch, color);
        }
    }

    fn put_centered(&mut self, center_x: usize, y: usize, text: &str, color: Color) {
        let width = text.chars().count();
        let start_x = center_x.saturating_sub(width / 2);
        self.put_text(start_x, y, text, color);
    }

    fn render(&self) -> String {
        let mut output = String::new();

        for y in 0..self.height {
            let mut current_color = Color::Default;

            for x in 0..self.width {
                let cell = self.cells[y * self.width + x];
                if cell.color != current_color {
                    output.push_str(cell.color.ansi());
                    current_color = cell.color;
                }
                output.push(cell.ch);
            }

            if current_color != Color::Default {
                output.push_str(RESET);
            }
            output.push('\n');
        }

        output
    }
}

#[derive(Debug, Clone, Copy)]
enum StickSide {
    Left,
    Right,
}

fn draw_bitmap_filled_button(
    canvas: &mut Canvas,
    center_x: usize,
    top_y: usize,
    pressed: bool,
    pixel_width: usize,
    pixel_height: usize,
) {
    let color = if pressed { Color::Green } else { Color::Gray };
    let bitmap = filled_rectangle_bitmap(pixel_width, pixel_height);
    let braille_lines = bitmap_to_braille(&bitmap_refs(&bitmap));

    draw_braille_lines(canvas, center_x, top_y, &braille_lines, color);
}

fn draw_bitmap_shoulder_button(
    canvas: &mut Canvas,
    center_x: usize,
    top_y: usize,
    label: &str,
    pressed: bool,
) {
    let color = if pressed { Color::Green } else { Color::Gray };
    let bitmap = shoulder_button_bitmap(24, 9);
    let braille_lines = bitmap_to_braille(&bitmap_refs(&bitmap));
    let button_width = braille_lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);

    draw_braille_lines(canvas, center_x, top_y + 1, &braille_lines, color);

    let label_y = top_y + 2;
    if label == "L1" {
        let label_x = center_x.saturating_sub(button_width / 2 + label.chars().count() + 1);
        canvas.put_text(label_x, label_y, label, color);
    } else {
        let label_x = center_x + button_width / 2 + 2;
        canvas.put_text(label_x, label_y, label, color);
    }
}

fn draw_bitmap_trigger_button(
    canvas: &mut Canvas,
    center_x: usize,
    top_y: usize,
    label: &str,
    value: u8,
) {
    let label_color = if value == u8::MAX {
        Color::Green
    } else {
        Color::Gray
    };
    let bitmap = trigger_button_bitmap(24, 18);
    let bitmap_refs = bitmap_refs(&bitmap);
    let braille_lines = bitmap_to_braille(&bitmap_refs);
    let button_width = braille_lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);

    draw_braille_fill(canvas, center_x, top_y + 1, &bitmap_refs, value);

    let label_y = top_y + 2;
    if label == "L2" {
        let label_x = center_x.saturating_sub(button_width / 2 + label.chars().count() + 1);
        canvas.put_text(label_x, label_y, label, label_color);
    } else {
        let label_x = center_x + button_width / 2 + 2;
        canvas.put_text(label_x, label_y, label, label_color);
    }
}

fn draw_bitmap_circle_button(
    canvas: &mut Canvas,
    center_x: usize,
    top_y: usize,
    label: &str,
    pressed: bool,
) {
    let color = if pressed { Color::Green } else { Color::Gray };
    let bitmap = circle_outline_bitmap(16, 12);
    let braille_lines = bitmap_to_braille(&bitmap_refs(&bitmap));
    let text_row = top_y + braille_lines.len() / 2;

    draw_braille_lines(canvas, center_x, top_y, &braille_lines, color);
    canvas.put_centered(center_x, text_row, label, color);
}

fn draw_bitmap_capsule_button(
    canvas: &mut Canvas,
    center_x: usize,
    top_y: usize,
    label: &str,
    pressed: bool,
) {
    let color = if pressed { Color::Green } else { Color::Gray };
    let bitmap = vertical_capsule_filled_bitmap(6, 12);
    let braille_lines = bitmap_to_braille(&bitmap_refs(&bitmap));

    canvas.put_centered(center_x, top_y, label, color);
    draw_braille_lines(canvas, center_x, top_y + 1, &braille_lines, color);
}

fn draw_dot_pattern(
    canvas: &mut Canvas,
    center_x: usize,
    top_y: usize,
    bitmap: &[&str],
    pressed: bool,
) {
    let color = if pressed { Color::Green } else { Color::Gray };
    let braille_lines = bitmap_to_braille(bitmap);
    draw_braille_lines(canvas, center_x, top_y, &braille_lines, color);
}

fn draw_bitmap_stick(
    canvas: &mut Canvas,
    center_x: usize,
    center_y: usize,
    x_raw: u8,
    y_raw: u8,
    pressed: bool,
    side: StickSide,
) {
    let border_color = if pressed { Color::Green } else { Color::Gray };
    let (marker_x, marker_y) = stick_marker_position(x_raw, y_raw);
    let pixels = stick_outline_pixel_grid(border_color, marker_x, marker_y);
    let char_width = STICK_PIXEL_DIAMETER.div_ceil(2);
    let char_height = STICK_PIXEL_DIAMETER.div_ceil(4);
    let top_y = center_y.saturating_sub(char_height / 2);

    draw_braille_color_grid(canvas, center_x, top_y, &pixels);

    match side {
        StickSide::Left => {
            let label_x = center_x.saturating_sub(char_width / 2 + 3);
            canvas.put_text(label_x, top_y, "L3", border_color)
        }
        StickSide::Right => {
            canvas.put_text(center_x + char_width / 2 + 1, top_y, "R3", border_color)
        }
    }
}

fn stick_marker_position(x_raw: u8, y_raw: u8) -> (usize, usize) {
    let max_index = (STICK_PIXEL_DIAMETER - 1) as f32;
    let x = normalized_x(x_raw);
    let y = normalized_y(y_raw);
    let radius = max_index / 2.0;
    let col = (radius + x * radius * 0.60).round() as usize;
    let row = (radius - y * radius * 0.60).round() as usize;
    (
        col.min(STICK_PIXEL_DIAMETER - 1),
        row.min(STICK_PIXEL_DIAMETER - 1),
    )
}

fn normalized_x(value: u8) -> f32 {
    (((value as f32) - 128.0) / 127.0).clamp(-1.0, 1.0)
}

fn normalized_y(value: u8) -> f32 {
    ((128.0 - (value as f32)) / 127.0).clamp(-1.0, 1.0)
}

fn stick_percent_x(value: u8) -> i16 {
    (normalized_x(value) * 100.0).round() as i16
}

fn stick_percent_y(value: u8) -> i16 {
    (normalized_y(value) * 100.0).round() as i16
}

#[cfg(test)]
fn trigger_percent(value: u8) -> u8 {
    (((value as f32) / 255.0) * 100.0).round() as u8
}

fn bitmap_to_braille(bitmap: &[&str]) -> Vec<String> {
    let pixel_width = bitmap
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    let pixel_height = bitmap.len();
    let char_width = pixel_width.div_ceil(2);
    let char_height = pixel_height.div_ceil(4);
    let mut lines = Vec::with_capacity(char_height);

    for char_y in 0..char_height {
        let mut line = String::with_capacity(char_width);

        for char_x in 0..char_width {
            let mut bits = 0u8;

            for pixel_y in 0..4 {
                for pixel_x in 0..2 {
                    let source_x = char_x * 2 + pixel_x;
                    let source_y = char_y * 4 + pixel_y;

                    if source_y >= pixel_height || !bitmap_pixel_is_on(bitmap, source_x, source_y) {
                        continue;
                    }

                    bits |= braille_bit(pixel_x, pixel_y);
                }
            }

            let braille = char::from_u32(0x2800 + u32::from(bits)).unwrap_or(' ');
            line.push(braille);
        }

        while line.ends_with('\u{2800}') {
            line.pop();
        }
        lines.push(line);
    }

    lines
}

fn bitmap_refs(rows: &[String]) -> Vec<&str> {
    rows.iter().map(String::as_str).collect()
}

fn draw_braille_lines(
    canvas: &mut Canvas,
    center_x: usize,
    top_y: usize,
    lines: &[String],
    color: Color,
) {
    let width = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    let start_x = center_x.saturating_sub(width / 2);

    for (row, line) in lines.iter().enumerate() {
        canvas.put_text(start_x, top_y + row, line, color);
    }
}

fn draw_braille_fill(
    canvas: &mut Canvas,
    center_x: usize,
    top_y: usize,
    bitmap: &[&str],
    value: u8,
) {
    let pixel_width = bitmap
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    let pixel_height = bitmap.len();
    let char_width = pixel_width.div_ceil(2);
    let char_height = pixel_height.div_ceil(4);
    let start_x = center_x.saturating_sub(char_width / 2);
    let fill_start_row =
        pixel_height.saturating_sub((usize::from(value) * pixel_height).div_ceil(255));

    for char_y in 0..char_height {
        for char_x in 0..char_width {
            let mut gray_bits = 0u8;
            let mut green_bits = 0u8;

            for pixel_y in 0..4 {
                for pixel_x in 0..2 {
                    let source_x = char_x * 2 + pixel_x;
                    let source_y = char_y * 4 + pixel_y;

                    if source_y >= pixel_height || !bitmap_pixel_is_on(bitmap, source_x, source_y) {
                        continue;
                    }

                    let bit = braille_bit(pixel_x, pixel_y);
                    if source_y >= fill_start_row {
                        green_bits |= bit;
                    } else {
                        gray_bits |= bit;
                    }
                }
            }

            let (bits, color) = if green_bits != 0 {
                (green_bits | gray_bits, Color::Green)
            } else if gray_bits != 0 {
                (gray_bits, Color::Gray)
            } else {
                continue;
            };

            let ch = char::from_u32(0x2800 + u32::from(bits)).unwrap_or(' ');
            canvas.put(start_x + char_x, top_y + char_y, ch, color);
        }
    }
}

fn put_centered_in_box(
    canvas: &mut Canvas,
    left_x: usize,
    box_width: usize,
    y: usize,
    text: &str,
    color: Color,
) {
    let text_width = text.chars().count();
    let x = left_x + box_width.saturating_sub(text_width) / 2;
    canvas.put_text(x, y, text, color);
}

fn draw_braille_color_grid(
    canvas: &mut Canvas,
    center_x: usize,
    top_y: usize,
    pixels: &[Vec<Option<Color>>],
) {
    let pixel_height = pixels.len();
    let pixel_width = pixels.first().map(Vec::len).unwrap_or(0);
    let char_width = pixel_width.div_ceil(2);
    let char_height = pixel_height.div_ceil(4);
    let start_x = center_x.saturating_sub(char_width / 2);

    for char_y in 0..char_height {
        for char_x in 0..char_width {
            let mut gray_bits = 0u8;
            let mut green_bits = 0u8;
            let mut cyan_bits = 0u8;

            for pixel_y in 0..4 {
                for pixel_x in 0..2 {
                    let source_x = char_x * 2 + pixel_x;
                    let source_y = char_y * 4 + pixel_y;
                    let Some(row) = pixels.get(source_y) else {
                        continue;
                    };
                    let Some(Some(color)) = row.get(source_x) else {
                        continue;
                    };

                    let bit = braille_bit(pixel_x, pixel_y);
                    match color {
                        Color::Gray => gray_bits |= bit,
                        Color::Green => green_bits |= bit,
                        Color::Cyan => cyan_bits |= bit,
                        Color::Default => {}
                    }
                }
            }

            let (bits, color) = if cyan_bits != 0 {
                (cyan_bits | green_bits | gray_bits, Color::Cyan)
            } else if green_bits != 0 {
                (green_bits | gray_bits, Color::Green)
            } else if gray_bits != 0 {
                (gray_bits, Color::Gray)
            } else {
                continue;
            };

            let ch = char::from_u32(0x2800 + u32::from(bits)).unwrap_or(' ');
            canvas.put(start_x + char_x, top_y + char_y, ch, color);
        }
    }
}

fn bitmap_pixel_is_on(bitmap: &[&str], x: usize, y: usize) -> bool {
    bitmap
        .get(y)
        .and_then(|line| line.chars().nth(x))
        .is_some_and(|ch| ch != ' ')
}

fn braille_bit(pixel_x: usize, pixel_y: usize) -> u8 {
    match (pixel_x, pixel_y) {
        (0, 0) => 0x01,
        (0, 1) => 0x02,
        (0, 2) => 0x04,
        (0, 3) => 0x40,
        (1, 0) => 0x08,
        (1, 1) => 0x10,
        (1, 2) => 0x20,
        (1, 3) => 0x80,
        _ => 0,
    }
}

fn filled_rectangle_bitmap(width: usize, height: usize) -> Vec<String> {
    vec!["#".repeat(width); height]
}

fn stick_outline_pixel_grid(
    border_color: Color,
    marker_x: usize,
    marker_y: usize,
) -> Vec<Vec<Option<Color>>> {
    let mut pixels = vec![vec![None; STICK_PIXEL_DIAMETER]; STICK_PIXEL_DIAMETER];
    let outline = stick_outline_bitmap();

    for (row_index, row_pattern) in outline.iter().enumerate() {
        for (col_index, ch) in row_pattern.chars().enumerate() {
            if ch == '#' {
                pixels[row_index][col_index] = Some(border_color);
            }
        }
    }

    let pointer_left = marker_x
        .saturating_sub(1)
        .min(STICK_PIXEL_DIAMETER.saturating_sub(2));
    let pointer_top = marker_y
        .saturating_sub(1)
        .min(STICK_PIXEL_DIAMETER.saturating_sub(2));

    for y in pointer_top..pointer_top + 2 {
        for x in pointer_left..pointer_left + 2 {
            pixels[y][x] = Some(Color::Cyan);
        }
    }

    pixels
}

fn stick_outline_bitmap() -> &'static [&'static str] {
    &[
        "           #######           ",
        "        #############        ",
        "      ####         ####      ",
        "    ####             ####    ",
        "   ###                 ###   ",
        "  ##                     ##  ",
        " ##                       ## ",
        "##                         ##",
        "##                         ##",
        "##                         ##",
        "##                         ##",
        "##                         ##",
        "##                         ##",
        "##                         ##",
        "##                         ##",
        "##                         ##",
        "##                         ##",
        "##                         ##",
        "##                         ##",
        "##                         ##",
        "##                         ##",
        " ##                       ## ",
        "  ##                     ##  ",
        "   ###                 ###   ",
        "    ####             ####    ",
        "      ####         ####      ",
        "        #############        ",
        "           #######           ",
    ]
}

fn shoulder_button_bitmap(width: usize, height: usize) -> Vec<String> {
    let mut rows = Vec::with_capacity(height);
    let center_x = (width as f32 - 1.0) / 2.0;
    let flat_height = height / 3;
    let curve_height = (height - flat_height).max(1) as f32;

    for y in 0..height {
        let mut row = String::with_capacity(width);
        if y < flat_height {
            row.push_str(&"#".repeat(width));
            rows.push(row);
            continue;
        }

        let dy = (y - flat_height) as f32 / curve_height;
        let half_width = center_x * (1.0 - dy * dy).max(0.0).sqrt();

        for x in 0..width {
            let dx = (x as f32 - center_x).abs();
            row.push(if dx <= half_width { '#' } else { ' ' });
        }

        rows.push(row);
    }

    rows
}

fn trigger_button_bitmap(width: usize, height: usize) -> Vec<String> {
    let mut rows = Vec::with_capacity(height);
    let center_x = (width as f32 - 1.0) / 2.0;
    let curve_height = height / 3;
    let flat_start = curve_height;
    let curve_height_f = curve_height.max(1) as f32;

    for y in 0..height {
        let mut row = String::with_capacity(width);
        if y >= flat_start {
            row.push_str(&"#".repeat(width));
            rows.push(row);
            continue;
        }

        let dy = (flat_start - y) as f32 / curve_height_f;
        let half_width = center_x * (1.0 - dy * dy).max(0.0).sqrt();

        for x in 0..width {
            let dx = (x as f32 - center_x).abs();
            row.push(if dx <= half_width { '#' } else { ' ' });
        }

        rows.push(row);
    }

    rows
}

fn circle_outline_bitmap(width: usize, height: usize) -> Vec<String> {
    let mut rows = Vec::with_capacity(height);
    let radius_x = (width as f32 - 1.0) / 2.0;
    let radius_y = (height as f32 - 1.0) / 2.0;

    for y in 0..height {
        let mut row = String::with_capacity(width);
        for x in 0..width {
            let dx = (x as f32 - radius_x) / radius_x;
            let dy = (y as f32 - radius_y) / radius_y;
            let value = dx * dx + dy * dy;
            row.push(if (0.72..=1.08).contains(&value) {
                '#'
            } else {
                ' '
            });
        }
        rows.push(row);
    }

    rows
}

fn vertical_capsule_filled_bitmap(width: usize, height: usize) -> Vec<String> {
    let mut rows = Vec::with_capacity(height);
    let radius_x = (width as f32 - 1.0) / 2.0;
    let radius_y = radius_x;
    let center_x = radius_x;
    let top_center_y = radius_y;
    let bottom_center_y = height as f32 - 1.0 - radius_y;

    for y in 0..height {
        let mut row = String::with_capacity(width);
        for x in 0..width {
            let xf = x as f32;
            let yf = y as f32;

            let value = if yf < top_center_y {
                let dx = (xf - center_x) / radius_x;
                let dy = (yf - top_center_y) / radius_y;
                dx * dx + dy * dy
            } else if yf > bottom_center_y {
                let dx = (xf - center_x) / radius_x;
                let dy = (yf - bottom_center_y) / radius_y;
                dx * dx + dy * dy
            } else {
                let dx = (xf - center_x) / radius_x;
                dx * dx
            };

            row.push(if value <= 1.0 { '#' } else { ' ' });
        }
        rows.push(row);
    }

    rows
}

fn dpad_up_bitmap() -> &'static [&'static str] {
    &[
        "##############",
        "##############",
        "##############",
        "##############",
        "##############",
        "##############",
        "##############",
        "##############",
        " ############ ",
        "  ##########  ",
        "   ########   ",
        "    ######    ",
        "      ##      ",
    ]
}

fn dpad_left_bitmap() -> &'static [&'static str] {
    &[
        "#########     ",
        "##########    ",
        "###########   ",
        "############  ",
        "############# ",
        "##############",
        "##############",
        "############# ",
        "############  ",
        "###########   ",
        "##########    ",
        "#########     ",
    ]
}

fn dpad_right_bitmap() -> &'static [&'static str] {
    &[
        "     #########",
        "    ##########",
        "   ###########",
        "  ############",
        " #############",
        "##############",
        "##############",
        " #############",
        "  ############",
        "   ###########",
        "    ##########",
        "     #########",
    ]
}

fn dpad_down_bitmap() -> &'static [&'static str] {
    &[
        "      ##      ",
        "    ######    ",
        "   ########   ",
        "  ##########  ",
        " ############ ",
        "##############",
        "##############",
        "##############",
        "##############",
        "##############",
        "##############",
        "##############",
        "##############",
    ]
}

fn triangle_bitmap() -> &'static [&'static str] {
    &[
        "        #       ",
        "       ###      ",
        "       ###      ",
        "      ## ##     ",
        "     ##   ##    ",
        "    ##     ##   ",
        "    ##     ##   ",
        "   ##       ##  ",
        "  ##         ## ",
        "  ##         ## ",
        " ###############",
        " ###############",
    ]
}

fn square_bitmap() -> &'static [&'static str] {
    &[
        "############",
        "############",
        "##        ##",
        "##        ##",
        "##        ##",
        "##        ##",
        "##        ##",
        "##        ##",
        "##        ##",
        "############",
        "############",
    ]
}

fn circle_bitmap() -> &'static [&'static str] {
    &[
        "   #######   ",
        "  #########  ",
        " ##       ## ",
        " ##       ## ",
        "##         ##",
        "##         ##",
        "##         ##",
        "##         ##",
        " ##       ## ",
        " ##       ## ",
        "  #########  ",
        "   #######   ",
    ]
}

fn cross_bitmap() -> &'static [&'static str] {
    &[
        "##          ##",
        "###        ###",
        " ###      ### ",
        "  ###    ###  ",
        "   ###  ###   ",
        "    ######    ",
        "    ######    ",
        "   ###  ###   ",
        "  ###    ###  ",
        " ###      ### ",
        "###        ###",
        "##          ##",
    ]
}

fn pad_visible_line(text: &str, width: usize) -> String {
    let visible_width = text.chars().count();
    if visible_width >= width {
        return text.to_owned();
    }

    format!("{text}{}", " ".repeat(width - visible_width))
}

fn write_colored_line(screen: &mut String, text: &str, color: &str) {
    let line = pad_visible_line(text, CANVAS_WIDTH);
    writeln!(screen, "{color}{line}{RESET}").expect("writing to String should not fail");
}

fn write_wrapped_colored_line(screen: &mut String, prefix: &str, text: &str, color: &str) {
    let prefix_width = prefix.chars().count();
    let indent = " ".repeat(prefix_width);
    let mut current = prefix.to_owned();
    let mut current_width = prefix_width;

    if text.is_empty() {
        write_colored_line(screen, prefix, color);
        return;
    }

    for token in text.split(' ') {
        let token_width = token.chars().count();
        let separator_width = usize::from(current_width > prefix_width);

        if current_width + separator_width + token_width > CANVAS_WIDTH {
            write_colored_line(screen, &current, color);
            current = indent.clone();
            current_width = prefix_width;
        }

        if current_width > prefix_width {
            current.push(' ');
            current_width += 1;
        }

        current.push_str(token);
        current_width += token_width;
    }

    write_colored_line(screen, &current, color);
}

fn format_report_hex(report: &[u8]) -> String {
    if report.is_empty() {
        return String::from("(none)");
    }

    report
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn centered_left_padding(terminal_width: usize, content_width: usize) -> usize {
    terminal_width.saturating_sub(content_width) / 2
}

fn fit_screen_to_terminal(screen: &str, content_width: usize) -> String {
    let Some((terminal_width, terminal_height)) = terminal_size() else {
        return screen.trim_end_matches('\n').to_owned();
    };

    let render_width = content_width.min(terminal_width);
    let left_padding = centered_left_padding(terminal_width, render_width);
    let padding = " ".repeat(left_padding);
    let mut fitted_lines = Vec::new();

    for line in screen.lines().take(terminal_height) {
        let clipped = truncate_ansi_line(line, render_width);
        if left_padding == 0 {
            fitted_lines.push(clipped);
        } else {
            fitted_lines.push(format!("{padding}{clipped}"));
        }
    }

    fitted_lines.join("\n")
}

fn truncate_ansi_line(line: &str, max_visible_width: usize) -> String {
    if max_visible_width == 0 {
        return String::new();
    }

    let mut output = String::new();
    let mut chars = line.chars().peekable();
    let mut visible_width = 0usize;
    let mut saw_escape = false;

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            saw_escape = true;
            output.push(ch);

            if let Some(next) = chars.next() {
                output.push(next);

                if next == '[' {
                    while let Some(csi_ch) = chars.next() {
                        output.push(csi_ch);
                        if ('@'..='~').contains(&csi_ch) {
                            break;
                        }
                    }
                }
            }

            continue;
        }

        if visible_width >= max_visible_width {
            break;
        }

        output.push(ch);
        visible_width += 1;
    }

    if saw_escape && !output.ends_with(RESET) {
        output.push_str(RESET);
    }

    output
}

fn terminal_size() -> Option<(usize, usize)> {
    if let Ok(columns) = env::var("COLUMNS") {
        if let Ok(lines) = env::var("LINES") {
            if let (Ok(width), Ok(height)) = (columns.parse::<usize>(), lines.parse::<usize>()) {
                if width > 0 && height > 0 {
                    return Some((width, height));
                }
            }
        }
    }

    terminal_size_from_ioctl()
}

#[cfg(unix)]
fn terminal_size_from_ioctl() -> Option<(usize, usize)> {
    use std::ffi::c_int;
    use std::ffi::c_ulong;
    use std::mem::MaybeUninit;

    #[repr(C)]
    struct WinSize {
        ws_row: u16,
        ws_col: u16,
        ws_xpixel: u16,
        ws_ypixel: u16,
    }

    unsafe extern "C" {
        fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
    }

    #[cfg(target_os = "macos")]
    const TIOCGWINSZ: c_ulong = 0x4008_7468;
    #[cfg(not(target_os = "macos"))]
    const TIOCGWINSZ: c_ulong = 0x5413;

    let stdout = io::stdout();
    let fd = stdout.as_raw_fd();
    let mut winsize = MaybeUninit::<WinSize>::uninit();
    let result = unsafe { ioctl(fd, TIOCGWINSZ, winsize.as_mut_ptr()) };
    if result != 0 {
        return None;
    }

    let winsize = unsafe { winsize.assume_init() };
    if winsize.ws_col == 0 || winsize.ws_row == 0 {
        return None;
    }

    Some((usize::from(winsize.ws_col), usize::from(winsize.ws_row)))
}

#[cfg(not(unix))]
fn terminal_size_from_ioctl() -> Option<(usize, usize)> {
    None
}

#[cfg(test)]
mod tests {
    use super::{
        centered_left_padding, fit_screen_to_terminal, format_report_hex, pad_visible_line,
        stick_marker_position, stick_percent_x, stick_percent_y, trigger_percent,
        truncate_ansi_line,
    };
    use std::env;

    #[test]
    fn stick_center_maps_to_grid_center() {
        assert_eq!(stick_marker_position(128, 128), (14, 14));
    }

    #[test]
    fn stick_edges_map_to_expected_percentages() {
        assert_eq!(stick_percent_x(255), 100);
        assert_eq!(stick_percent_x(0), -100);
        assert_eq!(stick_percent_y(0), 100);
        assert_eq!(stick_percent_y(255), -100);
    }

    #[test]
    fn trigger_percentage_covers_full_range() {
        assert_eq!(trigger_percent(0), 0);
        assert_eq!(trigger_percent(255), 100);
    }

    #[test]
    fn pad_visible_line_extends_shorter_text() {
        assert_eq!(pad_visible_line("abc", 5), "abc  ");
    }

    #[test]
    fn format_report_hex_handles_empty_reports() {
        assert_eq!(format_report_hex(&[]), "(none)");
    }

    #[test]
    fn centered_left_padding_uses_half_of_remaining_width() {
        assert_eq!(centered_left_padding(140, 120), 10);
    }

    #[test]
    fn truncate_ansi_line_respects_visible_width() {
        assert_eq!(
            truncate_ansi_line("\x1b[32mabcdef\x1b[0m", 3),
            "\x1b[32mabc\x1b[0m"
        );
    }

    #[test]
    fn fit_screen_to_terminal_limits_height_without_trailing_newline() {
        unsafe {
            env::set_var("COLUMNS", "10");
            env::set_var("LINES", "2");
        }
        let fitted = fit_screen_to_terminal("12345\n67890\nabcde\n", 5);
        assert_eq!(fitted, "  12345\n  67890");
    }
}
