// SPDX-License-Identifier: MIT

#[allow(dead_code)]
#[path = "../../../shared/dtls.rs"]
mod extracted_server_dtls;
#[allow(dead_code)]
#[path = "../../../shared/wire_protocol.rs"]
mod extracted_server_wire_protocol;

use crate::{
    dtls::DtlsClientHandshake,
    obfs::{ObfsCipher, ObfsMode},
    packet::PacketPool,
    protocol::{
        self, ConfigResponse, PANEL_RESTART_NOTICE, STREAM_ALIVE_PREFIX, STREAM_REPAIR_PREFIX,
    },
    wire_protocol as client_wire_protocol,
    wrap::derive_wrap_key,
};

const SERVER_PROTOCOL_SOURCE: &str = include_str!("../../../rust-server/protocol.rs");
const SERVER_MAIN_SOURCE: &str = include_str!("../../../rust-server/main.rs");

#[test]
fn server_contract_wire_identity_matches_extracted_server() {
    assert!(SERVER_MAIN_SOURCE.contains(
        r#"const WIRE_PROTOCOL_REVISION: &str = "CSQTT-WIRE-3";"#,
    ));
    assert_eq!(
        client_wire_protocol::PATH_ECHO_PREFIX,
        extracted_server_wire_protocol::PATH_ECHO_PREFIX,
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
        "pub(crate) const MAX_STREAM_WORKERS: usize = 100;",
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
            "wire-password",
            7,
            "wire-salt",
            1,
            Some(27),
        ),
        "GETCONF:46000|wire-password|7|wire-salt|1|27",
    );
    assert_eq!(
        protocol::disconnect_request("wire-password", "wire-salt"),
        "DISCONNECT:wire-password|wire-salt",
    );
    assert_eq!(
        protocol::workers_request("wire-password", 100),
        "GETWORKERS:wire-password|100",
    );
    assert_eq!(
        protocol::parse_tunworkers_response(b"TUNWORKERS:27:100"),
        Some((27, 100)),
    );
    assert_eq!(
        protocol::parse_config_response(b"NOCONF").unwrap(),
        ConfigResponse::NoConfig,
    );
    assert_eq!(
        protocol::parse_config_response(b"TUNCONF:10.66.67.2:1.1.1.1:46000:stream-v1").unwrap(),
        ConfigResponse::Config("TUNCONF:10.66.67.2:1.1.1.1:46000:stream-v1".to_owned()),
    );

    for control in [
        b"TUNCONF:10.66.67.2:1.1.1.1".as_slice(),
        b"NOCONF".as_slice(),
        b"DENIED:expired".as_slice(),
        b"READY_OK".as_slice(),
        b"OK:disconnected".as_slice(),
        b"TUNWORKERS:27:100".as_slice(),
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

    let count_offset = STREAM_REPAIR_PREFIX.len() + 8;
    payload[count_offset..count_offset + 2].copy_from_slice(&101u16.to_be_bytes());
    assert!(protocol::parse_stream_repair(&payload).is_none());
}

#[test]
fn server_contract_fragments_are_byte_identical_and_reassemble() {
    assert_eq!(client_wire_protocol::CSQTT_FRAG_MAGIC, 0xCF);
    assert_eq!(client_wire_protocol::FRAG_HEADER_LEN, 8);
    assert_eq!(client_wire_protocol::MAX_FRAG_PAYLOAD, 1272);
    assert_eq!(
        client_wire_protocol::MAX_PACKET_LEN,
        extracted_server_wire_protocol::MAX_PACKET_LEN,
    );
    let packet: Vec<u8> = (0..1500).map(|value| (value % 251) as u8).collect();
    let mut client_fragments = Vec::new();
    let mut server_fragments = Vec::new();
    client_wire_protocol::fragment_packet(0x1234, &packet, |fragment| {
        client_fragments.push(fragment.to_vec());
    });
    extracted_server_wire_protocol::fragment_packet(0x1234, &packet, |fragment| {
        server_fragments.push(fragment.to_vec());
    });
    assert_eq!(client_fragments, server_fragments);
    assert_eq!(client_fragments.len(), 2);
    assert_eq!(client_fragments[0].len(), 1280);
    assert_eq!(client_fragments[1].len(), 236);
    assert_eq!(
        &client_fragments[0][..8],
        &[0xCF, 0, 2, 0, 0x12, 0x34, 0x05, 0xDC],
    );

    let now = std::time::Instant::now();
    let mut server = extracted_server_wire_protocol::Reassembler::new();
    let mut server_packets = Vec::new();
    for index in [1, 1, 0] {
        server.process_fragment_from(42, &client_fragments[index], now, |payload| {
            server_packets.push(payload.to_vec());
        });
    }
    assert_eq!(server_packets, vec![packet.clone()]);

    let mut client = client_wire_protocol::Reassembler::new();
    let mut client_packets = Vec::new();
    for fragment in server_fragments.iter().rev() {
        client.process_fragment_from(42, fragment, now, |payload| {
            client_packets.push(payload.to_vec());
        });
    }
    assert_eq!(client_packets, vec![packet]);
}

#[test]
fn server_contract_dtls_handshake_and_application_data_roundtrip() {
    let mut client = DtlsClientHandshake::new().unwrap();
    let mut server = extracted_server_dtls::DtlsServerHandshake::new().unwrap();
    client.pin_fingerprint(server.certificate().fingerprint_sha256());

    let client_hello = client.start().unwrap();
    let server_flight = server.handle_datagram(&client_hello).unwrap().unwrap();
    let client_flight = client.handle_datagram(&server_flight).unwrap().unwrap();
    let server_finished = server.handle_datagram(&client_flight).unwrap().unwrap();
    client.handle_datagram(&server_finished).unwrap();
    assert!(client.is_connected());
    assert!(server.is_connected());

    let client_keys = client.srtp_keys().unwrap();
    let server_keys = server.srtp_keys().unwrap();
    assert_eq!(client_keys.client_cipher_key, server_keys.client_cipher_key);
    assert_eq!(client_keys.server_cipher_key, server_keys.server_cipher_key);
    assert_eq!(client_keys.client_write_key, server_keys.client_write_key);
    assert_eq!(client_keys.server_write_key, server_keys.server_write_key);
    assert_eq!(client_keys.client_write_salt, server_keys.client_write_salt);
    assert_eq!(client_keys.server_write_salt, server_keys.server_write_salt);

    let request = b"GETCONF:46000|wire-password|7|wire-salt|1|27";
    let wire = client.write_application_data(request).unwrap();
    server.handle_datagram(&wire).unwrap();
    assert_eq!(server.take_application_data(), vec![request.to_vec()]);

    let reply = b"TUNCONF:10.66.67.2:1.1.1.1:46000:stream-v1";
    let wire = server.write_application_data(reply).unwrap();
    client.handle_datagram(&wire).unwrap();
    assert_eq!(client.take_application_data(), vec![reply.to_vec()]);
}

#[test]
fn server_contract_published_audio_vector_unwraps() {
    const WIRE_HEX: &str = "b06f77881122334410203040bede0002325566775199aa005c468ea9f0329817714cb0d49b032855ae1b18c3ee81bf342bdd58cd166bdf23a16575b0ca29cb53618501b45b3ae4bb1fda630bb39ce45dbf5dde3073f99f87f2da0010b786331f3d88ba24fe8cf3be";
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
