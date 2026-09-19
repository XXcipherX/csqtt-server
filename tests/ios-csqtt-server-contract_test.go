// SPDX-License-Identifier: MIT

package csqtt

import (
	"bytes"
	"encoding/binary"
	"encoding/hex"
	"errors"
	"reflect"
	"testing"
)

func TestServerCompatibilityWireIdentity(t *testing.T) {
	if WireRevision != "CSQTT-WIRE-3" {
		t.Fatalf("wire revision = %q", WireRevision)
	}
	if LegacyWireRevision != "CSQTT-WIRE-2" {
		t.Fatalf("legacy wire revision = %q", LegacyWireRevision)
	}
	if MaxWorkers != 126 {
		t.Fatalf("max workers = %d", MaxWorkers)
	}
	if ReadyRequest != "READY" || ReadyOK != "READY_OK" {
		t.Fatalf("ready exchange = %q/%q", ReadyRequest, ReadyOK)
	}
	if NoConfigResponse != "NOCONF" || DisconnectedResponse != "OK:disconnected" {
		t.Fatalf("control replies = %q/%q", NoConfigResponse, DisconnectedResponse)
	}

	wantRestart := []byte("\xffCSQTT_PANEL_RESTART_V1\x00\x91\x7d\x03\xa8")
	if !bytes.Equal(PanelRestartNotice, wantRestart) {
		t.Fatalf("panel restart notice = %x", PanelRestartNotice)
	}
	if !bytes.Equal(StreamRepairPrefix, []byte("\xffCSQTT_STREAM_REPAIR_V1")) {
		t.Fatalf("stream repair prefix = %x", StreamRepairPrefix)
	}
	if !bytes.Equal(StreamAlivePrefix, []byte("\xffCSQTT_STREAM_ALIVE_V1")) {
		t.Fatalf("stream alive prefix = %x", StreamAlivePrefix)
	}
}

func TestServerCompatibilityControlPlane(t *testing.T) {
	want := "GETCONF:46000|wire-device|wire-password|7|wire-salt|1|27|CSQTT-WIRE-3"
	got := ConfigRequest("46000", "wire-device", "wire-password", 7, "wire-salt", 1, 27, WireRevision)
	if got != want {
		t.Fatalf("GETCONF = %q, want %q", got, want)
	}

	wantLegacy := "GETCONF:46000|wire-device|wire-password|7|wire-salt|1||CSQTT-WIRE-2"
	gotLegacy := ConfigRequest("46000", "wire-device", "wire-password", 7, "wire-salt", 1, 0, LegacyWireRevision)
	if gotLegacy != wantLegacy {
		t.Fatalf("legacy GETCONF = %q, want %q", gotLegacy, wantLegacy)
	}
	if got := DisconnectRequest("wire-device", "wire-salt"); got != "DISCONNECT:wire-device|wire-salt" {
		t.Fatalf("DISCONNECT = %q", got)
	}

	response := []byte("TUNCONF:10.66.67.2:1.1.1.1,8.8.8.8:46000:stream-v2")
	config, err := ParseConfigResponse(response)
	if err != nil {
		t.Fatalf("parse TUNCONF: %v", err)
	}
	if config.TunnelIP != "10.66.67.2" || config.DNS != "1.1.1.1,8.8.8.8" ||
		config.LocalPort != "46000" || config.StreamRevision != "stream-v2" || !config.FramesData() {
		t.Fatalf("parsed TUNCONF = %+v", config)
	}
	if !IsConfigResponse(response) || !IsControl(response) {
		t.Fatal("TUNCONF was not classified as control traffic")
	}

	if _, err := ParseConfigResponse([]byte(NoConfigResponse)); !errors.Is(err, ErrNoConfig) {
		t.Fatalf("NOCONF error = %v", err)
	}
	_, err = ParseConfigResponse([]byte("DENIED:password"))
	var denied *DeniedError
	if !errors.As(err, &denied) || denied.Reason != "password" {
		t.Fatalf("DENIED error = %#v", err)
	}

	if !IsPanelRestart(PanelRestartNotice) || !IsControl(PanelRestartNotice) {
		t.Fatal("panel restart notice was not classified as control traffic")
	}
	if !IsIdleKeepalive(bytes.Repeat([]byte{0xff}, 16)) || IsIdleKeepalive(nil) {
		t.Fatal("idle keepalive classification differs from the server")
	}
}

func TestServerCompatibilityStreamRepair(t *testing.T) {
	payload := append([]byte(nil), StreamRepairPrefix...)
	command := make([]byte, 11+3*2)
	binary.BigEndian.PutUint64(command[0:8], 42)
	binary.BigEndian.PutUint16(command[8:10], 12)
	command[10] = 3
	binary.BigEndian.PutUint16(command[11:13], 1)
	binary.BigEndian.PutUint16(command[13:15], 4)
	binary.BigEndian.PutUint16(command[15:17], 12)
	payload = append(payload, command...)

	parsed, ok := ParseStreamRepair(payload)
	if !ok {
		t.Fatal("valid stream repair command was rejected")
	}
	if parsed.Sequence != 42 || parsed.DesiredCount != 12 ||
		!reflect.DeepEqual(parsed.WorkerIDs, []uint16{1, 4, 12}) {
		t.Fatalf("parsed stream repair = %+v", parsed)
	}
	if !IsControl(payload) {
		t.Fatal("stream repair command was not classified as control traffic")
	}
}

func TestServerCompatibilityFlowFrame(t *testing.T) {
	header := FrameHeader{SenderID: 0x0102030405060708, FlowID: 0x1112131415161718, Sequence: 0x21222324}
	packet := make([]byte, FrameLen+3)
	if !header.Encode(packet) {
		t.Fatal("frame header encoding failed")
	}
	copy(packet[FrameLen:], []byte{0x45, 0x00, 0x00})
	if !bytes.Equal(packet[:4], []byte("CQF1")) || FrameLen != 24 {
		t.Fatalf("frame prefix = %x, length = %d", packet[:4], FrameLen)
	}
	decoded, payload, ok := DecodeFrame(packet)
	if !ok || decoded != header || !bytes.Equal(payload, []byte{0x45, 0x00, 0x00}) {
		t.Fatalf("decoded frame = %+v, payload = %x, ok = %v", decoded, payload, ok)
	}
}

func TestServerCompatibilityWrapVector(t *testing.T) {
	wire, err := hex.DecodeString("b06f77881122334410203040bede0002325566775199aa009e2a813c2df794354293ade7b0b8ef4aa6297cf3cc8b0a372342fbd678c10260dc543f8338262292992b12600eaa12d0d3155fa7179b62cd34ac41afb32db0ab7f6dced9e1bb0620daa26d1743f2c25af6bd9ade9901")
	if err != nil {
		t.Fatal(err)
	}
	key, err := DeriveKey("wire-password")
	if err != nil {
		t.Fatal(err)
	}
	cipher, err := NewCipher(key)
	if err != nil {
		t.Fatal(err)
	}
	plain, sequence, err := cipher.Unwrap(ModeAudio, wire)
	if err != nil {
		t.Fatalf("unwrap published server vector: %v", err)
	}
	want := []byte("GETCONF:46000|wire-device|wire-password|7|wire-salt|1|27|CSQTT-WIRE-2")
	if sequence != 0x7788 || !bytes.Equal(plain, want) {
		t.Fatalf("unwrapped sequence = %#x, payload = %q", sequence, plain)
	}
}
