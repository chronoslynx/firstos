use core::fmt;
use lazy_static::lazy_static;
use spin::Mutex;

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;
    interrupts::without_interrupts(|| {
        WRITER.lock().write_fmt(args).unwrap();
    });
}

#[macro_export]
macro_rules! eprint {
    ($($arg:tt)*) => ($crate::vga::_eprint(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! eprintln {
    () => ($crate::eprint!("\n"));
    ($($arg:tt)*) => ($crate::eprint!("{}\n", format_args!($($arg)*)));
}

pub fn _eprint(args: fmt::Arguments) {
    use core::fmt::Write;
    let mut wr = WRITER.lock();
    let orig_color = wr.color;
    wr.color = ColorCode::new(Color::White, Color::Red);
    wr.write_fmt(args).unwrap();
    wr.color = orig_color;
}

lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_position: 0,
        color: ColorCode::new(Color::Yellow, Color::Black),
    });
}

#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

#[repr(transparent)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct ColorCode(u8);

impl ColorCode {
    fn new(fg: Color, bg: Color) -> Self {
        Self((bg as u8) << 4 | (fg as u8))
    }
}

impl Into<u8> for ColorCode {
    fn into(self) -> u8 {
        self.0
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(C)]
struct Char {
    ascii: u8,
    color: ColorCode,
}

impl Into<u16> for Char {
    fn into(self) -> u16 {
        let color: u8 = self.color.into();
        ((color as u16) << 8) | (self.ascii as u16)
    }
}

impl From<u16> for Char {
    fn from(u: u16) -> Self {
        Char {
            ascii: (u & 0x00FF) as u8,
            color: ColorCode((u >> 8) as u8),
        }
    }
}

const VGA_BUFFER: *mut u16 = 0xb8000 as *mut _;

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

pub struct Writer {
    column_position: usize,
    color: ColorCode,
}

impl Writer {
    pub fn new() -> Self {
        Writer {
            column_position: 0,
            color: ColorCode::new(Color::Yellow, Color::Black),
        }
    }
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position > BUFFER_WIDTH {
                    self.new_line();
                }
                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;
                let offset = (row * BUFFER_WIDTH + col) as isize;
                let ch = Char {
                    ascii: byte,
                    color: self.color,
                };
                unsafe {
                    VGA_BUFFER.offset(offset).write_volatile(ch.into());
                }
                self.column_position += 1;
            }
        }
    }

    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // Printable range
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                _ => self.write_byte(0xfe),
            }
        }
    }

    fn new_line(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let offset = (row * BUFFER_WIDTH + col) as isize;
                unsafe {
                    let ch = VGA_BUFFER.offset(offset).read_volatile();
                    VGA_BUFFER
                        .offset(offset - BUFFER_WIDTH as isize)
                        .write_volatile(ch);
                }
            }
        }
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    fn clear_row(&mut self, row: usize) {
        let blank = Char {
            ascii: b' ',
            color: self.color,
        };
        let row_offset = row * BUFFER_WIDTH;
        for col in 0..BUFFER_WIDTH {
            let offset = (row_offset + col) as isize;
            unsafe {
                VGA_BUFFER.offset(offset).write_volatile(blank.into());
            }
        }
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

#[test_case]
fn test_char_from_to_u16() {
    let c = Char {
        ascii: b'!',
        color: ColorCode::new(Color::Red, Color::Blue),
    };
    let c16: u16 = c.into();
    assert_eq!(c, Char::from(c16));
}

#[test_case]
fn test_println() {
    println!("println should not panic");
}

#[test_case]
fn test_scrolling() {
    for _ in 0..200 {
        println!("test_scrolling output");
    }
}

#[test_case]
fn test_println_output() {
     use core::fmt::Write;
    use x86_64::instructions::interrupts;
    let s = "sentinel value";
    interrupts::without_interrupts(|| {
        let mut wr= WRITER.lock();
        writeln!(wr, "\n{}", s).expect("writeln failed");
        for (i, c) in s.chars().enumerate() {
            let offset = ((BUFFER_HEIGHT - 2) * BUFFER_WIDTH) + i;
            let s16 = unsafe { VGA_BUFFER.offset(offset as isize).read_volatile() };
            let screen_char = Char::from(s16).ascii;
            assert_eq!(char::from(screen_char), c);
        }
    });
}
