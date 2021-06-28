use super::buffer::{BytePacketBuffer, Result};
use super::packet::{Header, Packet, PacketType, QueryType, Question, Record, ResponseCode};
use std::{
    net::{Ipv4Addr, UdpSocket},
    str::FromStr,
};

pub fn start() -> Result<()> {
    let socket = UdpSocket::bind(("0.0.0.0", 34254))?;
    loop {
        match handle_query(&socket) {
            Ok(_) => {}
            Err(e) => eprintln!("error occured: {}", e),
        }
    }
}

fn handle_query(socket: &UdpSocket) -> Result<()> {
    let mut buffer = BytePacketBuffer::new_empty();
    let (_, src) = socket
        .recv_from(&mut buffer)
        .expect("didn't receive any data");

    let mut packet = Packet::new();
    packet.read(&mut buffer);

    // do some checks on packet
    if let Some(question) = packet.questions.first() {
        println!("question: {:#?}", question);
    } else {
        println!("no question found");
    }
    packet.additional.clear();
    packet.header.addi_c = 0;

    println!("packet is {:#?}", packet);

    let root_servers = vec!["198.41.0.4", "199.9.14.201"];
    let root_ipv4: Vec<Ipv4Addr> = root_servers
        .iter()
        .map(|x| Ipv4Addr::from_str(x).unwrap())
        .collect();

    let opt_response = resolve(socket, root_ipv4.as_slice(), &packet)?;

    // let mut req_buffer = BytePacketBuffer::new_empty();
    // packet.write(&mut req_buffer);

    // socket.send_to(&req_buffer[0..req_buffer.size], server)?;
    // //    socket.set_read_timeout(Some(Duration::from_secs(10)))?;
    // let mut response_buf = BytePacketBuffer::new_empty();
    // let (size, _) = socket.recv_from(&mut response_buf)?;
    // response_buf.size = size; // that's why it's a bad idea to allow Deref of the BytePacketBuffer (it gives ability to directly manipulate buffer, without changing size)

    println!("response: {:#?}", opt_response);
    if let Some(response) = opt_response {
        let mut response_buf = BytePacketBuffer::new_empty();
        response.write(&mut response_buf);
        socket.send_to(&response_buf[0..response_buf.size], src)?;
        if !response.answers.is_empty() {
            println!("answers: {:#?}", response.answers);
        }
    }
    // TODO: in case of failure, no proper response is being sent to the server

    // if !response_packet.authority.is_empty() {
    //     println!("authoritative: {:#?}", response_packet.authority);
    // }
    // if !response_packet.additional.is_empty() {
    //     println!("additional: {:#?}", response_packet.additional);
    // }

    Ok(())
}

fn resolve(
    socket: &UdpSocket,
    servers: &[Ipv4Addr],
    request_packet: &Packet,
) -> Result<Option<Packet>> {
    for server in servers.iter() {
        //println!("trying server {}", server);
        let pck = lookup(socket, *server, request_packet)?;
        //println!("packet {:#?}", pck);
        if !pck.answers.is_empty() {
            return Ok(Some(pck));
        }

        let servers: Vec<Ipv4Addr> = pck
            .additional
            .iter()
            .filter(|x| matches!(x, Record::A { .. }))
            .map(|server| match server {
                Record::A { ip, .. } => Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]),
                _ => panic!("should have been filtered out"),
            })
            .collect();
        //println!("servers size {}", servers.len());
        if !servers.is_empty() {
            return resolve(socket, servers.as_slice(), request_packet);
        }
    }

    Err("no servers remaining for request".into())
}

fn lookup(socket: &UdpSocket, server: Ipv4Addr, request_packet: &Packet) -> Result<Packet> {
    let mut req_buffer = BytePacketBuffer::new_empty();
    request_packet.write(&mut req_buffer);
    socket.send_to(&req_buffer[0..req_buffer.size], (server.to_string(), 53))?;
    let mut response_buf = BytePacketBuffer::new_empty();
    let (size, _) = socket.recv_from(&mut response_buf)?;
    response_buf.size = size; // that's why it's a bad idea to allow Deref of the BytePacketBuffer (it gives ability to directly manipulate buffer, without changing size)

    let mut response_packet = Packet::new();
    response_packet.read(&mut response_buf);

    Ok(response_packet)
}

pub fn create_request_packet(qname: &str, qtype: QueryType) -> Packet {
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
        name: qname.to_string(),
        qtype,
        class: 1,
    }];

    packet
}
