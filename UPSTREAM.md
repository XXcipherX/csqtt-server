# Исходный проект и протокол

Сервер основан на [CSQTT v2.5.0](https://github.com/amurcanov/csqtt/tree/v2.5.0), коммит [`fd936ef087c34cc46f26111dc2361f43b8b03c07`](https://github.com/amurcanov/csqtt/commit/fd936ef087c34cc46f26111dc2361f43b8b03c07).

## Модули

- `rust-server/` — сервер, веб-панель, Cargo.lock и сборочные скрипты;
- `shared/dtls.rs` — DTLS handshake, защита records и выработка ключей SRTP;
- `shared/wire_protocol.rs` — фрагментация и восстановление IP-пакетов;
- `app/src/main/assets/domains.txt` — список доменов, встраиваемый серверной веб-панелью;
- `scripts/server_asset_provenance.ps1` — проверка происхождения серверных бинарников.

Скрипты `rust-server/build_linux.*` формируют серверные артефакты, в том числе в каталоге `app/src/main/assets`. Для сборки standalone-контейнера используется корневой Dockerfile. Устройство компонентов описано в [ARCHITECTURE.md](ARCHITECTURE.md).

## Протокол 2.5.0

Запрос конфигурации:

```text
GETCONF:port|password|generation|salt|worker|desired_count
```

Сервер возвращает конфигурацию вида:

```text
TUNCONF:10.66.67.2:1.1.1.1:46000:stream-v1
```

Основные параметры:

| Параметр | Значение |
| --- | --- |
| Ревизия, возвращаемая `--protocol-revision` | `CSQTT-WIRE-3` |
| Максимальное число потоков | `100` |
| Magic фрагмента | `0xCF` |
| Размер заголовка фрагмента | `8` байт |
| Максимальный payload фрагмента | `1272` байта |
| Максимальный восстановленный пакет | `2048` байт |
| Обмен ключами | DTLS 1.2 / SRTP |

Константа `WIRE_PROTOCOL_REVISION` находится в `rust-server/main.rs`. Control plane также включает согласование числа потоков, stream repair, уведомления о перезапуске панели и проверку сетевого пути.

## Контракт с клиентом

`tests/upstream-csqtt-server-contract.rs` проверяет control plane, лимит потоков, stream repair, совпадение фрагментов, DTLS-обмен и опубликованный RTP vector. Fixture компилируется как модуль тестового бинарника клиента CSQTT v2.5.0.

Workflow `Upstream CSQTT 2.5.0 contract` использует коммит `fd936ef087c34cc46f26111dc2361f43b8b03c07`. Sparse checkout ограничен `rust-client/` и `shared/`; сборка выполняется под Linux GNU target.

Для ручного запуска из корня репозитория в Linux-среде с Rust 1.97.1:

```bash
git clone --depth 1 --filter=blob:none --sparse --branch v2.5.0 \
  https://github.com/amurcanov/csqtt.git .client-contract/upstream
git -C .client-contract/upstream sparse-checkout set rust-client shared
test "$(git -C .client-contract/upstream rev-parse HEAD)" = \
  fd936ef087c34cc46f26111dc2361f43b8b03c07
cp app/src/main/assets/domains.txt .client-contract/upstream/domains.txt
cp tests/upstream-csqtt-server-contract.rs \
  .client-contract/upstream/rust-client/server_contract_tests.rs
printf '\n#[cfg(test)]\nmod server_contract_tests;\n' >> \
  .client-contract/upstream/rust-client/main.rs
cd .client-contract/upstream/rust-client
native_target="$(rustc -vV | sed -n 's/^host: //p')"
cargo test --locked --target "$native_target" --bin client server_contract_
```

Команды рассчитаны на новый каталог `.client-contract/upstream`, который исключён из Git через `.gitignore`.

## Поддержка iOS

Поддержка протокола 2.5.0 в iOS-клиенте пока не подтверждена. Обсуждение: [anton48/vk-turn-proxy-ios#104](https://github.com/anton48/vk-turn-proxy-ios/issues/104).

## Лицензия

Корневой [LICENSE](LICENSE) содержит PolyForm Noncommercial License 1.0.0. В метаданных пакета `rust-server/Cargo.toml` указано `Unlicense`.
