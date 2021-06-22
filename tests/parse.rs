use druns::packet::{Packet, PacketType};
use druns::{buffer::BytePacketBuffer, packet::ResponseCode};

#[test]
fn test_parse_response_google() {
    let mut buffer = BytePacketBuffer::new(String::from("tests/google_response.txt"));
    let mut packet = Packet::new();
    packet.read(&mut buffer);
    assert_eq!(packet.header.ques_c, 1);
    assert_eq!(packet.header.ans_c, 1);
    assert_eq!(packet.header.rcode, ResponseCode::no_error);
    assert_eq!(packet.header.qr, PacketType::Response);
    print!("{:#?}", packet);
    assert_eq!(packet.questions[0].name, packet.answers[0].name);
    check_counts(&packet);
}

#[test]
fn test_parse_request_google() {
    let mut buffer = BytePacketBuffer::new(String::from("tests/google_request.txt"));
    let mut packet = Packet::new();
    packet.read(&mut buffer);
    assert_eq!(packet.header.ques_c, 1);
    assert_eq!(packet.header.ans_c, 0);
    assert_eq!(packet.header.rcode, ResponseCode::no_error);
    assert_eq!(packet.header.qr, PacketType::Query);

    check_counts(&packet);
}

#[test]
fn test_parse_request_netflix() {
    let mut buffer = BytePacketBuffer::new(String::from("tests/netflix_request.txt"));
    let mut packet = Packet::new();
    packet.read(&mut buffer);
    assert_eq!(packet.header.ques_c, 1);
    assert_eq!(packet.header.ans_c, 0);
    assert_eq!(packet.header.rcode, ResponseCode::no_error);
    assert_eq!(packet.header.qr, PacketType::Query);
    check_counts(&packet);
}

#[test]
fn test_parse_response_netflix() {
    let mut buffer = BytePacketBuffer::new(String::from("tests/netflix_response.txt"));
    let mut packet = Packet::new();
    packet.read(&mut buffer);
    assert_eq!(packet.header.ques_c, 1);
    assert_ne!(packet.header.ans_c, 0);
    assert_eq!(packet.header.rcode, ResponseCode::no_error);
    assert_eq!(packet.header.qr, PacketType::Response);
    check_counts(&packet);
}

fn check_counts(packet: &Packet) {
    assert_eq!(packet.header.ques_c, packet.questions.len() as u16);
    assert_eq!(packet.header.ans_c, packet.answers.len() as u16);
    assert_eq!(packet.header.auth_c, packet.authority.len() as u16);
    assert_eq!(packet.header.addi_c, packet.additional.len() as u16);
}
