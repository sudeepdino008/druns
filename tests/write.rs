use druns::buffer::{BytePacketBuffer, Result};
use druns::packet::Packet;

#[test]
fn test_write_string1() -> Result<()> {
    // start of buffer
    let mut buffer = BytePacketBuffer::new_empty();
    buffer.write_string("hello")?;
    assert_eq!(buffer.read_string_from(5, 0), "hello");
    Ok(())
}

#[test]
fn test_write_string2() -> Result<()> {
    // somewhere middle of buffer
    let mut buffer = BytePacketBuffer::new_empty();
    buffer.write_u32(123)?;
    buffer.write_u32(1223)?;
    buffer.write_u32(12123)?;
    buffer.write_u32(12333)?;
    let pos = (4 * 32) / 8;
    buffer.write_string("hello_world")?;
    assert_eq!(buffer.read_string_from(11, pos), "hello_world");
    Ok(())
}

#[test]
fn test_read_and_write1() -> Result<()> {
    let mut buffer = BytePacketBuffer::new(String::from("tests/google_request.txt"));
    let mut packet = Packet::new();
    packet.read(&mut buffer);

    let mut secondary_buffer = BytePacketBuffer::new_empty();
    packet.write(&mut secondary_buffer);

    let mut new_packet = Packet::new();
    new_packet.read(&mut secondary_buffer);
    assert_eq!(packet.header, new_packet.header);

    Ok(())
}

#[test]
fn test_read_and_write2() -> Result<()> {
    let mut buffer = BytePacketBuffer::new(String::from("tests/google_response.txt"));
    let mut packet = Packet::new();
    packet.read(&mut buffer);

    let mut secondary_buffer = BytePacketBuffer::new_empty();
    packet.write(&mut secondary_buffer);

    let mut new_packet = Packet::new();
    new_packet.read(&mut secondary_buffer);
    assert_eq!(packet.header, new_packet.header);

    Ok(())
}
