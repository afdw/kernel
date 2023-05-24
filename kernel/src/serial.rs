const PORT_BASE: u16 = 0x3F8;

const PORT_OFFSET_DATA: u16 = 0x0;
const PORT_OFFSET_INTERRUPT_ENABLE: u16 = 0x1;
const PORT_OFFSET_DIVISOR_LO: u16 = 0x0;
const PORT_OFFSET_DIVISOR_HI: u16 = 0x1;
const PORT_OFFSET_FIFO_CONTROL: u16 = 0x2;
const PORT_OFFSET_LINE_CONTROL: u16 = 0x3;
const PORT_OFFSET_MODEM_CONTROL: u16 = 0x4;
const PORT_OFFSET_LINE_STATUS: u16 = 0x5;

const LINE_STATUS_BIT_THRE: u8 = 1 << 5;

pub fn init() {
    unsafe {
        x86::io::outb(PORT_BASE + PORT_OFFSET_INTERRUPT_ENABLE, 0x00); // Disable all interrupts
        x86::io::outb(PORT_BASE + PORT_OFFSET_LINE_CONTROL, 0x80); // Enable DLAB (set baud rate divisor)
        x86::io::outb(PORT_BASE + PORT_OFFSET_DIVISOR_LO, 0x03); // Set divisor to 3 (lo byte) 38400 baud
        x86::io::outb(PORT_BASE + PORT_OFFSET_DIVISOR_HI, 0x00); // Set divisor to 3 (hi byte) 38400 baud
        x86::io::outb(PORT_BASE + PORT_OFFSET_LINE_CONTROL, 0x03); // 8 bits, no parity, one stop bit
        x86::io::outb(PORT_BASE + PORT_OFFSET_FIFO_CONTROL, 0x07); // Enable FIFO, clear them, with 1-byte threshold
        x86::io::outb(PORT_BASE + PORT_OFFSET_MODEM_CONTROL, 0x0B); // IRQs enabled, RTS/DSR set
        x86::io::outb(PORT_BASE + PORT_OFFSET_INTERRUPT_ENABLE, 0x01); // Interrupt on data available
    }
}

pub fn send(byte: u8) {
    unsafe {
        loop {
            let line_status = x86::io::inb(PORT_BASE + PORT_OFFSET_LINE_STATUS);
            if line_status & LINE_STATUS_BIT_THRE == 0 {
                continue;
            }
            break;
        }
        x86::io::outb(PORT_BASE + PORT_OFFSET_DATA, byte);
    }
}

pub struct Serial;

impl acid_io::Write for Serial {
    fn write(&mut self, src: &[u8]) -> acid_io::Result<usize> {
        for &byte in src {
            send(byte);
        }
        Ok(src.len())
    }

    fn flush(&mut self) -> acid_io::Result<()> {
        Ok(())
    }
}

impl core::fmt::Write for Serial {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        use acid_io::Write;
        self.write(s.as_bytes()).unwrap();
        Ok(())
    }
}
