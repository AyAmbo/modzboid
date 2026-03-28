use std::io::Cursor;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use crate::app_core::error::AppError;

const SERVERDATA_AUTH: i32 = 3;
const SERVERDATA_EXECCOMMAND: i32 = 2;

/// Build an RCON packet.
fn build_packet(id: i32, packet_type: i32, body: &str) -> Vec<u8> {
    let body_bytes = body.as_bytes();
    // length = 4 (id) + 4 (type) + body_len + 1 (null terminator) + 1 (pad)
    let length = 4 + 4 + body_bytes.len() as i32 + 1 + 1;

    let mut packet = Vec::with_capacity(length as usize + 4);
    packet.extend_from_slice(&length.to_le_bytes());
    packet.extend_from_slice(&id.to_le_bytes());
    packet.extend_from_slice(&packet_type.to_le_bytes());
    packet.extend_from_slice(body_bytes);
    packet.push(0); // null terminator
    packet.push(0); // pad
    packet
}

/// Read an RCON response packet. Returns (id, type, body).
async fn read_packet(stream: &mut TcpStream) -> Result<(i32, i32, String), AppError> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await
        .map_err(|e| AppError::Io(format!("RCON read length failed: {}", e)))?;
    let length = i32::from_le_bytes(len_buf) as usize;

    if length < 10 || length > 4096 {
        return Err(AppError::Io(format!("RCON invalid packet length: {}", length)));
    }

    let mut body_buf = vec![0u8; length];
    stream.read_exact(&mut body_buf).await
        .map_err(|e| AppError::Io(format!("RCON read body failed: {}", e)))?;

    let mut cursor = Cursor::new(&body_buf);
    let mut id_bytes = [0u8; 4];
    let mut type_bytes = [0u8; 4];
    std::io::Read::read_exact(&mut cursor, &mut id_bytes)
        .map_err(|e| AppError::Io(format!("RCON parse id failed: {}", e)))?;
    std::io::Read::read_exact(&mut cursor, &mut type_bytes)
        .map_err(|e| AppError::Io(format!("RCON parse type failed: {}", e)))?;

    let id = i32::from_le_bytes(id_bytes);
    let packet_type = i32::from_le_bytes(type_bytes);

    // Body is the rest minus the two null terminators
    let body_start = 8; // after id + type
    let body_end = if length >= 2 { length - 2 } else { body_start };
    let body = String::from_utf8_lossy(&body_buf[body_start..body_end]).to_string();

    Ok((id, packet_type, body))
}

/// Connect to an RCON server and authenticate.
pub async fn connect_and_auth(host: &str, port: u16, password: &str) -> Result<TcpStream, AppError> {
    let addr = format!("{}:{}", host, port);
    let mut stream = TcpStream::connect(&addr).await
        .map_err(|e| AppError::Io(format!("RCON connection failed to {}: {}", addr, e)))?;

    // Send auth packet
    let auth_packet = build_packet(1, SERVERDATA_AUTH, password);
    stream.write_all(&auth_packet).await
        .map_err(|e| AppError::Io(format!("RCON auth send failed: {}", e)))?;

    // Read auth response
    let (id, _type, _body) = read_packet(&mut stream).await?;

    if id == -1 {
        return Err(AppError::Validation("RCON authentication failed — wrong password".into()));
    }

    Ok(stream)
}

/// Send a command and receive the response.
pub async fn send_command(stream: &mut TcpStream, command: &str) -> Result<String, AppError> {
    let packet = build_packet(2, SERVERDATA_EXECCOMMAND, command);
    stream.write_all(&packet).await
        .map_err(|e| AppError::Io(format!("RCON command send failed: {}", e)))?;

    let (_id, _type, body) = read_packet(stream).await?;
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_packet() {
        let packet = build_packet(1, SERVERDATA_AUTH, "password");
        // length(4) + id(4) + type(4) + "password"(8) + null(1) + pad(1) = 18
        // first 4 bytes = length = 18
        let length = i32::from_le_bytes(packet[0..4].try_into().unwrap());
        assert_eq!(length, 18);

        let id = i32::from_le_bytes(packet[4..8].try_into().unwrap());
        assert_eq!(id, 1);

        let ptype = i32::from_le_bytes(packet[8..12].try_into().unwrap());
        assert_eq!(ptype, SERVERDATA_AUTH);

        // Body
        assert_eq!(&packet[12..20], b"password");
        // Null terminators
        assert_eq!(packet[20], 0);
        assert_eq!(packet[21], 0);
    }

    #[test]
    fn test_build_packet_empty() {
        let packet = build_packet(0, 2, "");
        let length = i32::from_le_bytes(packet[0..4].try_into().unwrap());
        assert_eq!(length, 10); // 4+4+0+1+1
    }
}
