use Record::A;

use super::buffer::{BytePacketBuffer, Result};
use std::{convert::TryInto, fmt::Debug, net::Ipv6Addr};

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
        buffer.reset_for_read();
        self.header.read_header(buffer);
        (0..self.header.ques_c).for_each(|_| {
            let mut question = Question::new();
            question.read(buffer);
            self.questions.push(question);
        });

        (0..self.header.ans_c).for_each(|_| {
            self.answers.push(Record::read(buffer).unwrap());
        });

        (0..self.header.auth_c).for_each(|_| {
            self.authority.push(Record::read(buffer).unwrap());
        });

        (0..self.header.addi_c).for_each(|_| {
            self.additional.push(Record::read(buffer).unwrap());
        });
    }
}

impl Packet {
    pub fn write(&self, buffer: &mut BytePacketBuffer) {
        self.header.write(buffer);
        self.questions.iter().for_each(|a| {
            a.write(buffer);
        });
        self.answers.iter().for_each(|a| {
            a.write(buffer).unwrap();
        });
        self.authority.iter().for_each(|a| {
            a.write(buffer).unwrap();
        });
        self.additional.iter().for_each(|a| {
            a.write(buffer).unwrap();
        });
    }
}

#[derive(Debug)]
pub struct Question {
    pub name: String,
    pub qtype: QueryType, // UNKNOWN type not handled
    pub class: u16,
}

impl Question {
    fn new() -> Question {
        Question {
            name: String::from(""),
            qtype: QueryType::UNKNOWN(0),
            class: 0,
        }
    }

    fn read(&mut self, buffer: &mut BytePacketBuffer) {
        self.name = buffer.read_qname();
        self.qtype = QueryType::from_num(buffer.read_u16().unwrap());
        self.class = buffer.read_u16().unwrap();
    }
}

impl Question {
    fn write(&self, buffer: &mut BytePacketBuffer) {
        let _ = buffer.write_qname(&self.name);
        let _ = buffer.write_u16(self.qtype.to_num());
        let _ = buffer.write_u16(self.class);
    }
}

pub enum QueryType {
    A,
    NS,
    CNAME,
    MX,
    AAAA,
    UNKNOWN(u16),
}

impl QueryType {
    pub fn to_num(&self) -> u16 {
        match *self {
            QueryType::A => 1,
            QueryType::NS => 2,
            QueryType::CNAME => 5,
            QueryType::MX => 15,
            QueryType::AAAA => 28,
            QueryType::UNKNOWN(x) => x,
        }
    }

    pub fn from_num(val: u16) -> QueryType {
        match val {
            1 => QueryType::A,
            2 => QueryType::NS,
            5 => QueryType::CNAME,
            15 => QueryType::MX,
            28 => QueryType::AAAA,
            x => QueryType::UNKNOWN(x),
        }
    }
}

impl Debug for QueryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_num().to_string())
    }
}

#[derive(Eq, Debug, PartialEq, Clone)]
pub enum Record {
    A {
        name: String,
        class: u16,
        ttl: u32,
        ip: [u8; 4],
    },
    NS {
        name: String,
        class: u16,
        ttl: u32,
        host: String,
    },
    CNAME {
        name: String,
        class: u16,
        host: String,
        ttl: u32,
    },
    MX {
        name: String,
        class: u16,
        priority: u16,
        host: String,
        ttl: u32,
    },
    AAAA {
        name: String,
        class: u16,
        ttl: u32,
        ip: Ipv6Addr,
    },
    UNKNOWN {
        name: String,
        rtype: u16,
        class: u16,
        ttl: u32,
        length: u16,
    },
}

impl Record {
    fn read(buffer: &mut BytePacketBuffer) -> Result<Record> {
        let name = buffer.read_qname();
        let rtype = buffer.read_u16().unwrap();
        let class = buffer.read_u16().unwrap();
        let ttl = buffer.read_u32().unwrap();
        let length = buffer.read_u16().unwrap();

        match rtype {
            1 => {
                let ip = buffer.read_u32().unwrap();

                Ok(A {
                    name,
                    class,
                    ttl,
                    ip: Record::parse_ip(ip),
                })
            }

            2 => {
                let host = buffer.read_qname();
                Ok(Record::NS {
                    name,
                    class,
                    ttl,
                    host,
                })
            }

            5 => {
                let host = buffer.read_qname();
                Ok(Record::CNAME {
                    name,
                    class,
                    host,
                    ttl,
                })
            }

            15 => {
                let priority = buffer.read_u16().unwrap();
                let host = buffer.read_qname();
                Ok(Record::MX {
                    name,
                    class,
                    priority,
                    host,
                    ttl,
                })
            }

            28 => Ok(Record::AAAA {
                name,
                class,
                ttl,
                ip: Ipv6Addr::new(
                    buffer.read_u16().unwrap(),
                    buffer.read_u16().unwrap(),
                    buffer.read_u16().unwrap(),
                    buffer.read_u16().unwrap(),
                    buffer.read_u16().unwrap(),
                    buffer.read_u16().unwrap(),
                    buffer.read_u16().unwrap(),
                    buffer.read_u16().unwrap(),
                ),
            }),

            _ => Ok(Record::UNKNOWN {
                name,
                rtype,
                class,
                ttl,
                length,
            }),
        }
    }

    fn parse_ip(ip: u32) -> [u8; 4] {
        [
            (ip >> 24).try_into().unwrap(),
            ((ip << 8) >> 24).try_into().unwrap(),
            ((ip << 16) >> 24).try_into().unwrap(),
            ((ip << 24) >> 24).try_into().unwrap(),
        ]
    }
}

impl Record {
    fn write(&self, buffer: &mut BytePacketBuffer) -> Result<usize> {
        match self {
            Record::A {
                name,
                class,
                ttl,
                ip,
            } => {
                buffer.write_qname(&name)?;
                buffer.write_u16(self.to_num())?;
                buffer.write_u16(*class)?;
                buffer.write_u32(*ttl)?;
                buffer.write_u16(4)?;
                buffer.write_u8(ip[0])?;
                buffer.write_u8(ip[1])?;
                buffer.write_u8(ip[2])?;
                buffer.write_u8(ip[3])?;
            }

            Record::NS {
                name,
                class,
                ttl,
                host,
            } => {
                buffer.write_qname(&name)?;
                buffer.write_u16(self.to_num())?;
                buffer.write_u16(*class)?;
                buffer.write_u32(*ttl)?;

                let pos = buffer.pos;
                buffer.write_u16(0)?;
                buffer.write_qname(&host)?;

                let size = buffer.pos - pos;
                buffer.set_u16(size as u16, pos)?;
            }

            Record::CNAME {
                name,
                class,
                host,
                ttl,
            } => {
                buffer.write_qname(&name)?;
                buffer.write_u16(self.to_num())?;
                buffer.write_u16(*class)?;
                buffer.write_u32(*ttl)?;

                let pos = buffer.pos;
                buffer.write_u16(0)?; //length
                buffer.write_qname(&host)?;
                let size = buffer.pos - pos - 2;
                buffer.set_u16(size as u16, pos)?;
            }

            Record::MX {
                name,
                class,
                priority,
                host,
                ttl,
            } => {
                buffer.write_qname(&name)?;
                buffer.write_u16(self.to_num())?;
                buffer.write_u16(*class)?;
                buffer.write_u32(*ttl)?;

                let pos = buffer.pos;
                buffer.write_u16(0)?;
                buffer.write_u16(*priority)?;
                buffer.write_qname(&host)?;

                let size = buffer.pos - pos - 2;
                buffer.set_u16(size as u16, pos)?;
            }

            Record::AAAA {
                name,
                class,
                ttl,
                ip,
            } => {
                buffer.write_qname(&name)?;
                buffer.write_u16(*class)?;
                buffer.write_u32(*ttl)?;
                ip.octets()
                    .iter()
                    .for_each(|x| buffer.write_u8(*x).unwrap());
            }

            Record::UNKNOWN {
                name,
                rtype,
                class,
                ttl,
                length,
            } => {
                buffer.write_qname(&name)?;
                buffer.write_u16(*rtype)?;
                buffer.write_u16(*class)?;
                buffer.write_u32(*ttl)?;
                buffer.write_u16(*length)?;
            }
        }

        Ok(0)
    }

    pub fn to_num(&self) -> u16 {
        match *self {
            Record::A { .. } => 1,
            Record::NS { .. } => 2,
            Record::CNAME { .. } => 5,
            Record::MX { .. } => 15,
            Record::AAAA { .. } => 28,
            Record::UNKNOWN { rtype, .. } => rtype,
        }
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
            rcode: ResponseCode::no_error,
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
        let _ = buffer.write_u16(self.id);

        let mut flags = u16::from(&self.qr) << 15;
        flags = ((flags >> 11) | self.opcode as u16) << 11;
        flags = ((flags >> 10) | self.to_u16(self.authoritative)) << 10;
        flags = ((flags >> 9) | self.to_u16(self.is_truncated)) << 9;
        flags = ((flags >> 8) | self.to_u16(self.recursion_desired)) << 8;
        flags = ((flags >> 7) | self.to_u16(self.recursion_available)) << 7;
        flags = ((flags >> 4) | self.reserved as u16) << 4;
        flags |= u16::from(&self.rcode);
        let _ = buffer.write_u16(flags);

        let _ = buffer.write_u16(self.ques_c);
        let _ = buffer.write_u16(self.ans_c);
        let _ = buffer.write_u16(self.auth_c);
        let _ = buffer.write_u16(self.addi_c);
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
#[allow(non_camel_case_types)]
pub enum ResponseCode {
    no_error, // no eror condition
    format_err,
    serv_fail,
    nx_domain,
    not_imp,
    refused,
    no_data,
}

impl From<u16> for ResponseCode {
    fn from(val: u16) -> ResponseCode {
        match val {
            0 => ResponseCode::no_error,
            1 => ResponseCode::format_err,
            2 => ResponseCode::serv_fail,
            3 => ResponseCode::nx_domain,
            4 => ResponseCode::not_imp,
            5 => ResponseCode::refused,
            6 => ResponseCode::no_data,
            _ => panic!("invalid response code"),
        }
    }
}

impl From<&ResponseCode> for u16 {
    fn from(val: &ResponseCode) -> u16 {
        match val {
            ResponseCode::no_error => 0,
            ResponseCode::format_err => 1,
            ResponseCode::serv_fail => 2,
            ResponseCode::nx_domain => 3,
            ResponseCode::not_imp => 4,
            ResponseCode::refused => 5,
            ResponseCode::no_data => 6,
        }
    }
}
