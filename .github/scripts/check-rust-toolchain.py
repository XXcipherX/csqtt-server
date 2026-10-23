"""Check that Docker and CI use the pinned server Rust toolchain."""

from pathlib import Path
import re
import sys
import tomllib


ROOT = Path(__file__).resolve().parents[2]
TOOLCHAIN = Path("rust-server/rust-toolchain.toml")


def main():
    toolchain = tomllib.loads((ROOT / TOOLCHAIN).read_text(encoding="utf-8"))
    version = toolchain["toolchain"]["channel"]
    if not re.fullmatch(r"\d+\.\d+\.\d+", version):
        print(f"::error file={TOOLCHAIN}::Rust must be pinned to an exact stable version")
        return 1

    errors = []
    dockerfile = (ROOT / "Dockerfile").read_text(encoding="utf-8")
    docker_version = re.search(r"^ARG RUST_VERSION=([^\s]+)$", dockerfile, re.MULTILINE)
    if docker_version is None or docker_version[1] != version:
        errors.append(("Dockerfile", f"RUST_VERSION must be {version}"))
    if not re.search(r"^FROM rust:\$\{RUST_VERSION\}-", dockerfile, re.MULTILINE):
        errors.append(("Dockerfile", "The Rust builder must use RUST_VERSION"))

    rust_steps = 0
    for workflow in sorted((ROOT / ".github/workflows").iterdir()):
        if workflow.suffix not in {".yml", ".yaml"}:
            continue
        source = workflow.read_text(encoding="utf-8")
        for ref in re.findall(r"uses:\s+dtolnay/rust-toolchain@([^\s#]+)", source):
            rust_steps += 1
            if ref != version:
                errors.append((workflow.relative_to(ROOT).as_posix(),
                               f"Rust toolchain {ref} must match {version}"))
    if rust_steps == 0:
        errors.append((".github/workflows", "No pinned Rust toolchain steps found"))

    manifest = tomllib.loads((ROOT / "rust-server/Cargo.toml").read_text(encoding="utf-8"))
    minimum = manifest["package"]["rust-version"]
    if tuple(map(int, minimum.split("."))) > tuple(map(int, version.split("."))):
        errors.append(("rust-server/Cargo.toml", "rust-version exceeds the pinned compiler"))

    for path, message in errors:
        print(f"::error file={path}::{message}")
    if errors:
        return 1
    print(f"Rust {version}: Docker and all {rust_steps} CI toolchain steps agree; MSRV {minimum}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
