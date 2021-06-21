use std::{convert::TryInto, fs, path::Path};

pub type Error = Box<dyn std::error::Error>;
pub type Result<T> = std::result::Result<T, Error>;

pub struct BytePacketBuffer {
    buffer: [u8; 512],
    pub pos: usize,
    pub size: usize,
}

impl std::ops::DerefMut for BytePacketBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // much better to have a size, which keeps track of buffer usage
        // rather than use pos, which can reset etc.
        &mut self.buffer
    }
}

impl std::ops::Deref for BytePacketBuffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.buffer[0..self.size]
    }
}

impl BytePacketBuffer {
    pub fn new(from_file: String) -> BytePacketBuffer {
        let path = Path::new(&from_file);
        let contents = fs::read(path);
        let contents = contents.expect(&format!("error reading contents from file {}", from_file));

        let mut buffer = [0; 512];
        (0..contents.len()).for_each(|i| {
            buffer[i] = contents[i];
        });

        BytePacketBuffer {
            buffer,
            pos: 0,
            size: contents.len(),
        }
    }

    pub fn new_empty() -> BytePacketBuffer {
        BytePacketBuffer {
            buffer: [0; 512],
            pos: 0,
            size: 0,
        }
    }

    pub fn reset_for_read(&mut self) {
        self.pos = 0;
    }
}

// reading from buffer
impl BytePacketBuffer {
    pub fn read_u8(&mut self) -> Result<u8> {
        let result = self.read_u8_from(self.pos);
        self.pos += 1;
        result
    }

    pub fn read_u8_from(&mut self, cpos: usize) -> Result<u8> {
        if cpos >= 511 {
            Err("overflow".into())
        } else {
            Ok(self.buffer[cpos])
        }
    }

    pub fn read_u16(&mut self) -> Result<u16> {
        let result = self.read_u16_from(self.pos);
        self.pos += 2;
        result
    }

    pub fn read_u16_from(&mut self, mut cpos: usize) -> Result<u16> {
        if cpos >= 510 {
            return Err("overflow".into());
        }
        let mut result: u16 = self.read_u8_from(cpos).unwrap().into();
        cpos += 1;
        let sresult: u16 = self.read_u8_from(cpos).unwrap().into();
        result = (result << 8) + sresult;
        Ok(result)
    }

    pub fn read_u32(&mut self) -> Result<u32> {
        let mut result: u32 = self.read_u16().unwrap().into();
        let sresult: u32 = self.read_u16().unwrap().into();
        result = (result << 16) + sresult;
        Ok(result)
    }

    pub fn read_string(&mut self, length: u8) -> String {
        let result = self.read_string_from(length, self.pos);
        self.pos += length as usize;
        result
    }

    pub fn read_string_from(&mut self, length: u8, mut cpos: usize) -> String {
        let mut vv: Vec<u8> = vec![];
        (0..length).for_each(|_| {
            vv.push(self.read_u8_from(cpos).unwrap());
            cpos += 1;
        });
        String::from_utf8(vv).unwrap_or_else(|err| format!("utf8 error {}", err))
    }

    pub fn read_qname(&mut self) -> String {
        let length = self.read_u8().unwrap();
        if length & 0xC0 != 0 {
            // jump directive
            let following_byte = self.read_u8().unwrap() as u16;
            let msb_removed_length = (length ^ 0xC0) as u16;
            let jmp_position: u16 = following_byte + (msb_removed_length << 8);
            self.read_qname_from(jmp_position as usize).0
        } else {
            let (qname, new_pos) = self.read_qname_from(self.pos);
            self.pos = new_pos;
            qname
        }
    }

    pub fn read_qname_from(&mut self, mut cpos: usize) -> (String, usize) {
        let mut qname = String::new();
        while {
            let length = self.read_u8_from(cpos).unwrap();
            cpos += 1;
            qname = qname + &self.read_string_from(length, cpos);
            cpos += length as usize;
            if length != 0 {
                qname += ".";
                true
            } else {
                false
            }
        } {}
        (qname, cpos)
    }
}

// writing to the buffer
impl BytePacketBuffer {
    pub fn write_u8(&mut self, val: u8) -> Result<()> {
        if self.pos > 511 {
            return Err("buffer overflow".into());
        }
        self.buffer[self.pos] = val;
        self.pos += 1;
        self.size = self.pos;
        Ok(())
    }

    pub fn write_u16(&mut self, val: u16) -> Result<()> {
        let first_u8: u8 = (val >> 8).try_into().unwrap();
        let second_u8: u8 = ((val << 8) >> 8).try_into().unwrap();
        self.write_u8(first_u8)?;
        self.write_u8(second_u8)?;
        self.size = self.pos;
        Ok(())
    }

    pub fn write_u32(&mut self, val: u32) -> Result<()> {
        let first_u16: u16 = (val >> 16).try_into().unwrap();
        let second_u16: u16 = ((val << 16) >> 16).try_into().unwrap();
        self.write_u16(first_u16)?;
        self.write_u16(second_u16)?;
        self.size = self.pos;
        Ok(())
    }

    pub fn write_string(&mut self, val: &str) -> Result<()> {
        if val.is_empty() {
            return Ok(());
        }
        if self.pos + val.len() > 511 {
            return Err("buffer overflow".into());
        }
        let destination_slice = &mut self.buffer[self.pos..self.pos + val.len()];
        destination_slice.copy_from_slice(val.as_bytes());
        self.size = self.pos;
        Ok(())
    }

    pub fn write_qname(&mut self, val: &str) -> Result<()> {
        // TODO: implement jump directive here
        if val.is_empty() {
            return Ok(());
        }

        val.split('.').for_each(|label| {
            self.write_u8(label.len().try_into().unwrap());
            self.write_string(label);
        });

        self.size = self.pos;
        Ok(())
    }
}
