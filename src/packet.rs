use Record::A;

use super::buffer::{BytePacketBuffer, Result};
use std::{convert::TryInto, fmt::Debug, net::UdpSocket, time::Duration};

pub fn start() -> Result<()> {
    let server = "8.8.8.8:53";

    let packet = create_request_packet();

    println!("the request packet is {:#?}", packet);
    let mut buffer = BytePacketBuffer::new_empty();
    packet.write(&mut buffer);
    let socket = UdpSocket::bind(("0.0.0.0", 34254))?;

    socket.send_to(&buffer[0..buffer.size], server)?;
    socket.set_read_timeout(Some(Duration::from_secs(10)))?;
    let mut response_buf = BytePacketBuffer::new_empty();
    let _ = socket.recv_from(&mut response_buf[0..512]).unwrap();
    let mut response_packet = Packet::new();
    response_packet.read(&mut response_buf);
    println!("response packet is {:#?}", response_packet);

    Ok(())
}

pub fn create_request_packet() -> Packet {
    let mut packet = Packet::new();
    let header = Header {
        id: 8378,
        qr: PacketType::Query,
        opcode: 0,
        authoritative: false,
        is_truncated: false,
        recursion_desired: true,
        recursion_available: false,
        reserved: 2,
        rcode: ResponseCode::no_error,
        ques_c: 1,
        ans_c: 0,
        auth_c: 0,
        addi_c: 0,
    };
    packet.header = header;
    packet.questions = vec![Question {
        name: "yahoo.com.".to_string(),
        qtype: 15,
        class: 1,
    }];

    packet
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
    pub qtype: u16, // UNKNOWN type not handled
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
        self.name = buffer.read_qname();
        self.qtype = buffer.read_u16().unwrap();
        self.class = buffer.read_u16().unwrap();
    }
}

impl Question {
    fn write(&self, buffer: &mut BytePacketBuffer) {
        let _ = buffer.write_qname(&self.name);
        let _ = buffer.write_u16(self.qtype);
        let _ = buffer.write_u16(self.class);
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
            Record::CNAME { .. } => 5,
            Record::MX { .. } => 15,
            Record::UNKNOWN { rtype, .. } => rtype,
        }
    }
}

// pub struct Record {
//     pub name: String,
//     //    pub qtype: u16,
//     pub class: u16,
//     pub ttl: u32,
//     pub length: u16,
//     //  pub ip: u32,
// }

// impl Debug for Record {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         let ip_arr = self.parse_ip();
//         f.debug_struct("Record")
//             .field("name", &self.name)
//             .field("qtype", &self.qtype)
//             .field("class", &self.class)
//             .field("ttl", &self.ttl)
//             .field("length", &self.length)
//             .field(
//                 "ip",
//                 &format!("{}.{}.{}.{}", ip_arr[0], ip_arr[1], ip_arr[2], ip_arr[3]),
//             )
//             .finish()
//     }
// }

// impl Record {
//     fn new() -> Record {
//         Record {
//             name: String::new(),
//             qtype: 0,
//             class: 0,
//             ttl: 0,
//             length: 0,
//             ip: 0,
//         }
//     }

//     fn read(&mut self, buffer: &mut BytePacketBuffer) {
//         self.name = buffer.read_qname();
//         self.qtype = buffer.read_u16().unwrap();
//         self.class = buffer.read_u16().unwrap();
//         self.ttl = buffer.read_u32().unwrap();
//         self.length = buffer.read_u16().unwrap();
//         self.ip = buffer.read_u32().unwrap();
//     }

//     fn parse_ip(&self) -> [u8; 4] {
//         [
//             (self.ip >> 24).try_into().unwrap(),
//             ((self.ip << 8) >> 24).try_into().unwrap(),
//             ((self.ip << 16) >> 24).try_into().unwrap(),
//             ((self.ip << 24) >> 24).try_into().unwrap(),
//         ]
//     }
// }

// impl Record {
//     fn write(&self, buffer: &mut BytePacketBuffer) {
//         let _ = buffer.write_qname(&self.name);
//         let _ = buffer.write_u16(self.qtype);
//         let _ = buffer.write_u16(self.class);
//         let _ = buffer.write_u32(self.ttl);
//         let _ = buffer.write_u16(self.length);
//         let _ = buffer.write_u32(self.ip);
//     }
// }

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
            _ => panic!("unhandled response code {:?}", val),
        }
    }
}
