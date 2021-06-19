use super::buffer::BytePacketBuffer;
use std::{convert::TryInto, fmt::Debug};
use PacketType::{Query, Response};

pub fn start() {
    let mut buffer = BytePacketBuffer::new(String::from("response.txt"));
    let mut packet = Packet::new();
    packet.read(&mut buffer);
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

    pub fn read(&mut self, buffer: &mut BytePacketBuffer) {
        self.header.read_header(buffer);
        (0..self.header.ques_c).for_each(|_| {
            let mut question = Question::new();
            question.read(buffer);
            self.questions.push(question);
        });

        (0..self.header.ans_c).for_each(|_| {
            let mut answer = Record::new();
            answer.read(buffer);
            self.answers.push(answer);
        });

        (0..self.header.auth_c).for_each(|_| {
            let mut auth = Record::new();
            auth.read(buffer);
            self.authority.push(auth);
        });

        (0..self.header.addi_c).for_each(|_| {
            let mut addi = Record::new();
            addi.read(buffer);
            self.additional.push(addi);
        });
    }
}

impl Packet {
    pub fn write(&self, buffer: &mut BytePacketBuffer) {
        self.header.write(buffer);
        self.questions.iter().for_each(|q| q.write(buffer));
        self.answers.iter().for_each(|a| a.write(buffer));
        self.authority.iter().for_each(|a| a.write(buffer));
        self.additional.iter().for_each(|a| a.write(buffer));
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

    fn read(&mut self, buffer: &mut BytePacketBuffer) {
        // read the domain name
        let mut domain_name = String::new();
        while {
            let length = buffer.read_u8().unwrap();
            let temp_name = buffer.read_string(length);
            domain_name = domain_name + &temp_name + if temp_name.is_empty() { "" } else { "." };
            length != 0
        } {}
        self.name = domain_name;

        // qtype
        self.qtype = buffer.read_u16().unwrap();
        // class
        self.class = buffer.read_u16().unwrap();
    }
}

impl Question {
    fn write(&self, buffer: &mut BytePacketBuffer) {
        self.name.split('.').for_each(|label| {
            let len = label.len().try_into().unwrap();
            buffer.write_u8(len);
            buffer.write_string(label);
        });

        buffer.write_u16(self.qtype);
        buffer.write_u16(self.class);
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

    fn read(&mut self, buffer: &mut BytePacketBuffer) {
        self.name = buffer.read_qname();
        self.qtype = buffer.read_u16().unwrap();
        self.class = buffer.read_u16().unwrap();
        self.ttl = buffer.read_u32().unwrap();
        self.length = buffer.read_u16().unwrap();
        self.ip = buffer.read_u32().unwrap();
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

impl Record {
    fn write(&self, buffer: &mut BytePacketBuffer) {
        buffer.write_qname(&self.name);
        buffer.write_u16(self.qtype);
        buffer.write_u16(self.class);
        buffer.write_u32(self.ttl);
        buffer.write_u16(self.length);
        buffer.write_u32(self.ip);
    }
}

#[derive(Debug, PartialEq)]
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
        self.id = buffer.read_u16().unwrap();
        let flags = buffer.read_u16().unwrap();
        self.qr = ((flags >> 15) & 1).into();
        self.opcode = ((flags << 1) >> 12) as u8;
        self.authoritative = (flags << 5) >> 15 == 1;
        self.is_truncated = (flags << 6) >> 15 == 1;
        self.recursion_desired = (flags << 7) >> 15 == 1;
        self.recursion_available = (flags << 8) >> 15 == 1;
        self.reserved = ((flags << 9) >> 13) as u8;
        self.rcode = ((flags << 12) >> 12).into();

        self.ques_c = buffer.read_u16().unwrap();
        self.ans_c = buffer.read_u16().unwrap();
        self.auth_c = buffer.read_u16().unwrap();
        self.addi_c = buffer.read_u16().unwrap();
    }
}

impl Header {
    fn write(&self, buffer: &mut BytePacketBuffer) {
        print!("{}", format!("writing header -- id {}", self.id));
        buffer.write_u16(self.id);

        let mut flags = u16::from(&self.qr) << 15;
        flags = ((flags >> 11) | self.opcode as u16) << 11;
        flags = ((flags >> 10) | self.to_u16(self.authoritative)) << 10;
        flags = ((flags >> 9) | self.to_u16(self.is_truncated)) << 9;
        flags = ((flags >> 8) | self.to_u16(self.recursion_desired)) << 8;
        flags = ((flags >> 7) | self.to_u16(self.recursion_available)) << 7;
        flags = ((flags >> 6) | self.reserved as u16) << 6;
        flags = flags | u16::from(&self.rcode);
        buffer.write_u16(flags);

        buffer.write_u16(self.ques_c);
        buffer.write_u16(self.ans_c);
        buffer.write_u16(self.auth_c);
        buffer.write_u16(self.addi_c);
    }

    fn to_u16(&self, val: bool) -> u16 {
        if val {
            1
        } else {
            0
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum PacketType {
    Query,    // 0
    Response, // 1
}

impl From<u16> for PacketType {
    fn from(val: u16) -> Self {
        match val {
            0 => PacketType::Query,
            1 => PacketType::Response,
            _ => panic!("unexpected value"),
        }
    }
}

impl From<&PacketType> for u16 {
    fn from(val: &PacketType) -> Self {
        match val {
            PacketType::Query => 0,
            PacketType::Response => 1,
        }
    }
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

impl From<u16> for ResponseCode {
    fn from(val: u16) -> ResponseCode {
        match val {
            0 => ResponseCode::NoError,
            1 => ResponseCode::FormatErr,
            2 => ResponseCode::ServFail,
            3 => ResponseCode::NxDomain,
            4 => ResponseCode::NotImp,
            5 => ResponseCode::Refused,
            6 => ResponseCode::NoData,
            _ => panic!(format!("unknown response code {}", val)),
        }
    }
}

impl From<&ResponseCode> for u16 {
    fn from(val: &ResponseCode) -> u16 {
        match val {
            NoError => 0,
            FormatErr => 1,
            ServFail => 2,
            NxDoman => 3,
            NotImp => 4,
            Refused => 5,
            NoData => 6,
        }
    }
}

// impl Into<ResponseCode> for u16 {
//     fn into(self) -> ResponseCode {
//         ResponseCode::NoError
//     }
// }
