use super::buffer::BytePacketBuffer;
use std::{convert::TryInto, fmt::Debug};
use PacketType::{Query, Response};

pub fn start() {
    let mut buffer = BytePacketBuffer::new(String::from("response.txt"));
    let mut packet = Packet::new();
    packet.read_packet(&mut buffer);
    print!("the packet is {:#?}", packet);
}

#[derive(Debug)]
pub struct Packet {
    pub header: Header,
    pub questions: Vec<Question>,
    pub answers: Vec<Record>,
    pub authority: Vec<Record>,
    pub additional: Vec<Record>,
}

impl Packet {
    pub fn new() -> Packet {
        Packet {
            header: Header::new(),
            questions: vec![],
            answers: vec![],
            authority: vec![],
            additional: vec![],
        }
    }

    pub fn read_packet(&mut self, buffer: &mut BytePacketBuffer) {
        self.header.read_header(buffer);
        (0..self.header.ques_c).for_each(|_| {
            let mut question = Question::new();
            question.read_question(buffer);
            self.questions.push(question);
        });

        (0..self.header.ans_c).for_each(|_| {
            let mut answer = Record::new();
            answer.read_record(buffer);
            self.answers.push(answer);
        });

        (0..self.header.auth_c).for_each(|_| {
            let mut auth = Record::new();
            auth.read_record(buffer);
            self.authority.push(auth);
        });

        (0..self.header.addi_c).for_each(|_| {
            let mut addi = Record::new();
            addi.read_record(buffer);
            self.additional.push(addi);
        });
    }
}

#[derive(Debug)]
pub struct Question {
    pub name: String,
    pub qtype: u16,
    pub class: u16,
}

impl Question {
    fn new() -> Question {
        Question {
            name: String::from(""),
            qtype: 0,
            class: 0,
        }
    }

    fn read_question(&mut self, buffer: &mut BytePacketBuffer) {
        // read the domain name
        let mut domain_name = String::new();
        while {
            let length = buffer.read_byte().unwrap();
            let temp_name = buffer.read_string(length);
            domain_name = domain_name + &temp_name + if temp_name.is_empty() { "" } else { "." };
            length != 0
        } {}
        self.name = domain_name;

        // qtype
        self.qtype = buffer.read_2bytes().unwrap();
        // class
        self.class = buffer.read_2bytes().unwrap();
    }
}

pub struct Record {
    pub name: String,
    pub qtype: u16,
    pub class: u16,
    pub ttl: u32,
    pub length: u16,
    pub ip: u32,
}

impl Debug for Record {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ip_arr = self.parse_ip();
        f.debug_struct("Record")
            .field("name", &self.name)
            .field("qtype", &self.qtype)
            .field("class", &self.class)
            .field("ttl", &self.ttl)
            .field("length", &self.length)
            .field(
                "ip",
                &format!("{}.{}.{}.{}", ip_arr[0], ip_arr[1], ip_arr[2], ip_arr[3]),
            )
            .finish()
    }
}

impl Record {
    fn new() -> Record {
        Record {
            name: String::new(),
            qtype: 0,
            class: 0,
            ttl: 0,
            length: 0,
            ip: 0,
        }
    }

    fn read_record(&mut self, buffer: &mut BytePacketBuffer) {
        while {
            let length = buffer.read_byte().unwrap();
            if length & 0xC0 != 0 {
                // jump directive
                let following_byte = buffer.read_byte().unwrap() as u16;
                let msb_removed_length = (length ^ 0xC0) as u16;
                let jmp_position: u16 = following_byte + (msb_removed_length << 8);
                self.name = self.read_name(buffer, jmp_position as usize);
            }

            length != 0
        } {}

        self.qtype = buffer.read_2bytes().unwrap();
        self.class = buffer.read_2bytes().unwrap();
        self.ttl = buffer.read_4bytes().unwrap();
        self.length = buffer.read_2bytes().unwrap();
        self.ip = buffer.read_4bytes().unwrap();
    }

    fn read_name(&mut self, buffer: &mut BytePacketBuffer, mut cpos: usize) -> String {
        let mut domain_name = String::new();
        while {
            let length = buffer.read_byte_from(cpos).unwrap();
            cpos += 1;
            domain_name = domain_name + &buffer.read_string_from(length, cpos);
            cpos += length as usize;
            if length != 0 {
                domain_name += ".";
                true
            } else {
                false
            }
        } {}
        domain_name
    }

    fn parse_ip(&self) -> [u8; 4] {
        [
            (self.ip >> 24).try_into().unwrap(),
            ((self.ip << 8) >> 24).try_into().unwrap(),
            ((self.ip << 16) >> 24).try_into().unwrap(),
            ((self.ip << 24) >> 24).try_into().unwrap(),
        ]
    }
}

#[derive(Debug)]
pub struct Header {
    pub id: u16,
    pub qr: PacketType,
    pub opcode: u8, // 4 bits
    pub authoritative: bool,
    pub is_truncated: bool,
    pub recursion_desired: bool,
    pub recursion_available: bool,
    pub reserved: u8, //3 bits
    pub rcode: ResponseCode,
    pub ques_c: u16,
    pub ans_c: u16,
    pub auth_c: u16,
    pub addi_c: u16,
}

impl Header {
    fn new() -> Header {
        Header {
            id: 0,
            qr: PacketType::Query,
            opcode: 0,
            authoritative: false,
            is_truncated: false,
            recursion_desired: false,
            recursion_available: false,
            reserved: 0,
            rcode: ResponseCode::NoError,
            ques_c: 0,
            ans_c: 0,
            auth_c: 0,
            addi_c: 0,
        }
    }

    fn read_header(&mut self, buffer: &mut BytePacketBuffer) {
        self.id = buffer.read_2bytes().unwrap();
        let flags = buffer.read_2bytes().unwrap();
        self.qr = if (flags >> 15) & 1 == 0 {
            Query
        } else {
            Response
        };

        self.opcode = ((flags << 1) >> 12) as u8;
        self.authoritative = (flags << 5) >> 15 == 1;
        self.is_truncated = (flags << 6) >> 15 == 1;
        self.recursion_desired = (flags << 7) >> 15 == 1;
        self.recursion_available = (flags << 8) >> 15 == 1;
        self.reserved = ((flags << 9) >> 13) as u8;
        self.rcode = Header::get_rcode(((flags << 12) >> 12) as u8);

        self.ques_c = buffer.read_2bytes().unwrap();
        self.ans_c = buffer.read_2bytes().unwrap();
        self.auth_c = buffer.read_2bytes().unwrap();
        self.addi_c = buffer.read_2bytes().unwrap();
    }

    fn get_rcode(val: u8) -> ResponseCode {
        if val == 0 {
            ResponseCode::NoError
        } else if val == 1 {
            ResponseCode::FormatErr
        } else if val == 2 {
            ResponseCode::ServFail
        } else if val == 3 {
            ResponseCode::NxDomain
        } else if val == 4 {
            ResponseCode::NotImp
        } else if val == 5 {
            ResponseCode::Refused
        } else if val == 6 {
            ResponseCode::NoData
        } else {
            panic!("unknown response code val");
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum PacketType {
    Query,    // 0
    Response, // 1
}

#[derive(Debug, PartialEq)]
pub enum ResponseCode {
    NoError, // no eror condition
    FormatErr,
    ServFail,
    NxDomain,
    NotImp,
    Refused,
    NoData,
}
