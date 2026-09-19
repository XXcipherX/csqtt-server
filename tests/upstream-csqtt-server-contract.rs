// SPDX-License-Identifier: MIT

#[path = "../../../shared/flow_frame.rs"]
mod extracted_server_flow_frame;
#[path = "../../../shared/wire_protocol.rs"]
mod extracted_server_wire_protocol;

use crate::{
    flow_frame::{self as client_flow_frame, FrameHeader as ClientFrameHeader},
    obfs::{ObfsCipher, ObfsMode},
    packet::PacketPool,
    protocol::{
        self, ConfigResponse, PANEL_RESTART_NOTICE, STREAM_ALIVE_PREFIX, STREAM_REPAIR_PREFIX,
    },
    wire_protocol as client_wire_protocol,
    wrap::derive_wrap_key,
};

const SERVER_PROTOCOL_SOURCE: &str = include_str!("../../../rust-server/protocol.rs");

#[test]
fn server_contract_wire_identity_matches_extracted_server() {
    assert_eq!(
        client_wire_protocol::WIRE_PROTOCOL_REVISION,
        extracted_server_wire_protocol::WIRE_PROTOCOL_REVISION,
    );
    assert_eq!(
        client_wire_protocol::LEGACY_WIRE_PROTOCOL_REVISION,
        extracted_server_wire_protocol::LEGACY_WIRE_PROTOCOL_REVISION,
    );
    assert_eq!(client_wire_protocol::WIRE_PROTOCOL_REVISION, "CSQTT-WIRE-3");
    assert_eq!(
        client_wire_protocol::LEGACY_WIRE_PROTOCOL_REVISION,
        "CSQTT-WIRE-2",
    );

    assert_eq!(
        PANEL_RESTART_NOTICE,
        b"\xffCSQTT_PANEL_RESTART_V1\x00\x91\x7d\x03\xa8",
    );
    assert_eq!(STREAM_REPAIR_PREFIX, b"\xffCSQTT_STREAM_REPAIR_V1");
    assert_eq!(STREAM_ALIVE_PREFIX, b"\xffCSQTT_STREAM_ALIVE_V1");

    for server_literal in [
        r#"pub const PANEL_RESTART_NOTICE: &[u8] = b"\xffCSQTT_PANEL_RESTART_V1\x00\x91\x7d\x03\xa8";"#,
        r#"const STREAM_REPAIR_PREFIX: &[u8] = b"\xffCSQTT_STREAM_REPAIR_V1";"#,
        r#"const STREAM_ALIVE_PREFIX: &[u8] = b"\xffCSQTT_STREAM_ALIVE_V1";"#,
        "pub(crate) const MAX_STREAM_WORKERS: usize = 126;",
    ] {
        assert!(
            SERVER_PROTOCOL_SOURCE.contains(server_literal),
            "extracted server no longer exposes expected protocol literal: {server_literal}",
        );
    }
}

#[test]
fn server_contract_control_plane_matches_extracted_server() {
    assert_eq!(
        protocol::config_request(
            "46000",
            "wire-device",
            "wire-password",
            7,
            "wire-salt",
            1,
            Some(27),
        ),
        "GETCONF:46000|wire-device|wire-password|7|wire-salt|1|27|CSQTT-WIRE-3",
    );
    assert_eq!(
        protocol::disconnect_request("wire-device", "wire-salt"),
        "DISCONNECT:wire-device|wire-salt",
    );
    assert_eq!(
        protocol::parse_config_response(b"NOCONF").unwrap(),
        ConfigResponse::NoConfig,
    );
    assert_eq!(
        protocol::parse_config_response(b"TUNCONF:10.66.67.2:1.1.1.1").unwrap(),
        ConfigResponse::Config("TUNCONF:10.66.67.2:1.1.1.1".to_owned()),
    );

    for control in [
        b"TUNCONF:10.66.67.2:1.1.1.1".as_slice(),
        b"NOCONF".as_slice(),
        b"DENIED:expired".as_slice(),
        b"READY_OK".as_slice(),
        b"OK:disconnected".as_slice(),
        PANEL_RESTART_NOTICE,
        STREAM_REPAIR_PREFIX,
        STREAM_ALIVE_PREFIX,
    ] {
        assert!(protocol::is_control_response(control));
    }

    for server_reply in ["b\"READY_OK\"", "b\"OK:disconnected\"", "\"NOCONF\""] {
        assert!(
            SERVER_PROTOCOL_SOURCE.contains(server_reply),
            "extracted server no longer contains reply {server_reply}",
        );
    }
}

#[test]
fn server_contract_stream_repair_matches_extracted_server() {
    let mut payload = STREAM_REPAIR_PREFIX.to_vec();
    payload.extend_from_slice(&42u64.to_be_bytes());
    payload.extend_from_slice(&12u16.to_be_bytes());
    payload.push(3);
    payload.extend_from_slice(&1u16.to_be_bytes());
    payload.extend_from_slice(&4u16.to_be_bytes());
    payload.extend_from_slice(&12u16.to_be_bytes());

    let parsed = protocol::parse_stream_repair(&payload).expect("valid repair command");
    assert_eq!(parsed.sequence, 42);
    assert_eq!(parsed.desired_count, 12);
    assert_eq!(parsed.worker_ids, [1, 4, 12]);
    assert!(protocol::is_control_response(&payload));
}

#[test]
fn server_contract_flow_frame_is_byte_identical() {
    let client_header = ClientFrameHeader {
        sender_id: 0x0102_0304_0506_0708,
        flow_id: 0x1112_1314_1516_1718,
        sequence: 0x2122_2324,
    };
    let server_header = extracted_server_flow_frame::FrameHeader {
        sender_id: client_header.sender_id,
        flow_id: client_header.flow_id,
        sequence: client_header.sequence,
    };
    let mut client_packet = vec![0u8; client_flow_frame::FRAME_LEN + 3];
    let mut server_packet = vec![0u8; extracted_server_flow_frame::FRAME_LEN + 3];
    assert!(client_header.encode(&mut client_packet));
    assert!(server_header.encode(&mut server_packet));
    client_packet[client_flow_frame::FRAME_LEN..].copy_from_slice(&[0x45, 0x00, 0x00]);
    server_packet[extracted_server_flow_frame::FRAME_LEN..].copy_from_slice(&[0x45, 0x00, 0x00]);
    assert_eq!(client_packet, server_packet);
    assert_eq!(&client_packet[..4], b"CQF1");

    let (decoded, payload) = ClientFrameHeader::decode(&server_packet).expect("client frame decode");
    assert_eq!(decoded, client_header);
    assert_eq!(payload, [0x45, 0x00, 0x00]);
}

#[test]
fn server_contract_published_audio_vector_unwraps() {
    const WIRE_HEX: &str = "b06f77881122334410203040bede0002325566775199aa009e2a813c2df794354293ade7b0b8ef4aa6297cf3cc8b0a372342fbd678c10260dc543f8338262292992b12600eaa12d0d3155fa7179b62cd34ac41afb32db0ab7f6dced9e1bb0620daa26d1743f2c25af6bd9ade9901";
    const PLAIN: &[u8] =
        b"GETCONF:46000|wire-device|wire-password|7|wire-salt|1|27|CSQTT-WIRE-2";

    assert_eq!(
        extracted_server_wire_protocol::CLIENT_GETCONF_AUDIO_FIXTURE,
        WIRE_HEX,
    );
    let wire = hex::decode(WIRE_HEX).unwrap();
    let pool = PacketPool::new(1);
    let mut packet = pool.acquire();
    packet.read_area()[..wire.len()].copy_from_slice(&wire);
    packet.set_read_len(wire.len()).unwrap();

    let cipher = ObfsCipher::new(derive_wrap_key("wire-password").unwrap()).unwrap();
    let sequence = cipher.unwrap(&mut packet, ObfsMode::Audio).unwrap();
    assert_eq!(sequence, 0x7788);
    assert_eq!(packet.as_slice(), PLAIN);
}
