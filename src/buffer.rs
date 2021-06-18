use std::{fs, path::Path};

pub struct BytePacketBuffer {
    buffer: [u8; 512],
    pos: usize,
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

        BytePacketBuffer { buffer, pos: 0 }
    }

    pub fn read_byte(&mut self) -> Result<u8, ()> {
        let result = self.read_byte_from(self.pos);
        self.pos += 1;
        result
    }

    pub fn read_byte_from(&mut self, cpos: usize) -> Result<u8, ()> {
        if cpos >= 511 {
            Err(())
        } else {
            Ok(self.buffer[cpos])
        }
    }

    pub fn read_2bytes(&mut self) -> Result<u16, ()> {
        let result = self.read_2bytes_from(self.pos);
        self.pos += 2;
        result
    }

    pub fn read_2bytes_from(&mut self, mut cpos: usize) -> Result<u16, ()> {
        if cpos >= 510 {
            return Err(());
        }
        let mut result: u16 = self.read_byte_from(cpos).unwrap().into();
        cpos += 1;
        let sresult: u16 = self.read_byte_from(cpos).unwrap().into();
        result = (result << 8) + sresult;
        Ok(result)
    }

    pub fn read_4bytes(&mut self) -> Result<u32, ()> {
        let mut result: u32 = self.read_2bytes().unwrap().into();
        let sresult: u32 = self.read_2bytes().unwrap().into();
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
            vv.push(self.read_byte_from(cpos).unwrap());
            cpos += 1;
        });
        String::from_utf8(vv).unwrap_or_else(|err| format!("utf8 error {}", err))
    }
}
