# csqtt-server 2.5.0

[![Server CI](https://github.com/XXcipherX/csqtt-server/actions/workflows/server-ci.yml/badge.svg?branch=2.5.0)](https://github.com/XXcipherX/csqtt-server/actions/workflows/server-ci.yml)
[![Docker CI](https://github.com/XXcipherX/csqtt-server/actions/workflows/docker-ci.yml/badge.svg?branch=2.5.0)](https://github.com/XXcipherX/csqtt-server/actions/workflows/docker-ci.yml)
[![Upstream CSQTT 2.5.0 contract](https://github.com/XXcipherX/csqtt-server/actions/workflows/upstream-csqtt-contract.yml/badge.svg?branch=2.5.0)](https://github.com/XXcipherX/csqtt-server/actions/workflows/upstream-csqtt-contract.yml)

Сервер [CSQTT v2.5.0](https://github.com/amurcanov/csqtt/tree/v2.5.0) на Rust для туннелирования IP-трафика через TURN/RTP. Включает TUN dataplane и встроенную веб-панель управления.

В репозитории находятся сервер, общие модули протокола, Dockerfile, диагностические скрипты и тесты совместимости. Описание протокола и проверок — в [UPSTREAM.md](UPSTREAM.md), устройство сервера — в [ARCHITECTURE.md](ARCHITECTURE.md).

## Возможности

- туннелирование IP-трафика через TURN/RTP, поддержка DTLS 1.2 и SRTP;
- фрагментация пакетов с заголовком `0xCF` и восстановление пакетов до 2048 байт;
- до 100 потоков на клиентское подключение;
- TUN-интерфейс, пакетный UDP-ввод-вывод и восстановление dataplane после ошибок;
- встроенная HTTP/HTTPS-панель: клиенты, группы, правила маршрутизации, DNS, локальные proxy profiles, статистика и журналы;
- постоянное хранение базы SQLite, конфигурации и TLS-материалов;
- Dockerfile для сборки под `linux/amd64` и `linux/arm64`.

## Сборка Docker image

Команды ниже предназначены для Linux-сервера с уже установленным Docker. Для сборки образа Rust и Go на хосте не нужны: Rust и сборочные зависимости находятся внутри build-контейнера.

```bash
git clone --branch 2.5.0 --single-branch https://github.com/XXcipherX/csqtt-server.git
cd csqtt-server
docker build --tag csqtt-server:2.5.0 .
docker run --rm csqtt-server:2.5.0 --version
docker run --rm csqtt-server:2.5.0 --protocol-revision
```

Проверки должны вывести `csqtt 2.5.0` и `CSQTT-WIRE-3`. Обычная Docker-сборка создаёт образ для архитектуры хоста. При наличии Docker Buildx образ для одной выбранной платформы можно собрать так:

```bash
docker buildx build --platform linux/arm64 --load --tag csqtt-server:2.5.0 .
```

Для `amd64` используйте `--platform linux/amd64`. Кросс-сборке нужен подходящий builder с поддержкой выбранной платформы.

Docker CI собирает образы без публикации. Workflow `Build and push Docker image` публикует multi-platform image в `ghcr.io/xxcipherx/csqtt-server` вручную или при push тега `v2.5.*`.

При ручном запуске укажите ветку `2.5.0` и тег образа в формате `2.5.x`. Значение по умолчанию — `2.5.0`.

## Запуск на VPS

Контейнер использует host networking. На VPS должны быть доступны `/dev/net/tun`, IPv4 forwarding, NAT для сети `10.66.67.0/24` и разрешающие правила firewall для серверных портов.

Создайте постоянный каталог данных `/opt/csqtt-docker/data` и файл `/etc/csqtt-server.env` с собственными паролями и внешним IPv4-адресом сервера:

```dotenv
CSQTT_MAIN_PASSWORD=your-connection-password
CSQTT_WEB_USER=admin
CSQTT_WEB_PASS=your-panel-password
CSQTT_PUBLIC_IP=203.0.113.10
```

Пример запуска собранного образа:

```bash
docker run --detach \
  --name csqtt \
  --restart unless-stopped \
  --network host \
  --device /dev/net/tun:/dev/net/tun \
  --cap-add NET_ADMIN \
  --cap-add NET_RAW \
  --env-file /etc/csqtt-server.env \
  --mount type=bind,source=/opt/csqtt-docker/data,target=/etc/csqtt \
  csqtt-server:2.5.0
```

Все данные сервера сохраняются в `/etc/csqtt` внутри контейнера. При обновлении подключайте тот же постоянный каталог; перед переходом с предыдущей версии сохраните его резервную копию.

### Порты по умолчанию

| Назначение | Протокол | Порт |
| --- | --- | ---: |
| CSQTT peer | UDP | `46000` |
| Веб-панель | TCP | `46002` |

Панель открывается по адресу `https://SERVER_IP:46002/`. При автоматически созданном TLS-сертификате браузер может сообщить о недоверенном сертификате. В режиме host networking колонка `PORTS` в `docker ps` остаётся пустой.

### Основные параметры

| Аргумент | Переменная окружения | Назначение |
| --- | --- | --- |
| `--listen` | — | UDP-адрес, по умолчанию `0.0.0.0:46000` |
| `--web-port` | — | Порт панели, по умолчанию `46002` |
| `--config-dir` | — | Каталог данных, по умолчанию `/etc/csqtt` |
| `--password` | `CSQTT_MAIN_PASSWORD` | Основной пароль подключения |
| `--web-user` | `CSQTT_WEB_USER` | Логин панели, по умолчанию `admin` |
| `--web-pass` | `CSQTT_WEB_PASS` | Пароль панели |
| `--public-ip` | `CSQTT_PUBLIC_IP` | Внешний IPv4-адрес для клиентских ссылок |
| `--dns` | `CSQTT_DNS` | Один или два DNS IPv4-адреса через запятую |
| `--web-secret-path` | `CSQTT_WEB_SECRET_PATH` | Префикс пути панели |
| `--secure-cookie` | `CSQTT_SECURE_COOKIE` | Выдача cookie только по HTTPS |

Полный список аргументов: `docker run --rm csqtt-server:2.5.0 --help`.

## Совместимость

Для подключения используйте клиент CSQTT из тега `v2.5.0` проекта `amurcanov/csqtt`. Сервер сообщает ревизию протокола `CSQTT-WIRE-3`; форматы сообщений и пакетов описаны в [UPSTREAM.md](UPSTREAM.md).

Поддержка 2.5.0 в iOS-клиенте пока не подтверждена; обсуждение ведётся в [anton48/vk-turn-proxy-ios#104](https://github.com/anton48/vk-turn-proxy-ios/issues/104).

## Автоматические проверки

- `Server CI` — форматирование, Clippy, unit/protocol tests, release build, проверка версии и ревизии протокола, `cargo audit`;
- `Docker CI` — сборка образов для `linux/amd64` и native `linux/arm64` без публикации;
- `Upstream CSQTT 2.5.0 contract` — проверка контракта с клиентом из исходного коммита тега `v2.5.0`;
- `Workflow lint` — проверка GitHub Actions через actionlint;
- `Dependency Review` — проверка изменений зависимостей в pull requests;
- `Build and push Docker image` — отдельная публикация multi-platform image.

Проверки запускаются при push в `2.5.0`, в pull requests с целевой веткой `2.5.0` и вручную с учётом настроек каждого workflow. Все сборочные инструменты для CI устанавливаются на GitHub runners.

## Ручные проверки и диагностика

Unit- и протокольные тесты входят в оригинальные Rust-модули. В отдельной Linux-среде с готовым Rust 1.97.1 можно запустить:

```bash
cd rust-server
native_target="$(rustc -vV | sed -n 's/^host: //p')"
cargo fmt --all -- --check
cargo clippy --locked --target "$native_target" --all-targets -- -D warnings
cargo test --locked --target "$native_target" --all-targets
cargo build --locked --release --target "$native_target" --bin csqtt
"./target/$native_target/release/csqtt" --protocol-revision
```

Явный `--target` выбирает native Linux target вместо musl target из оригинального `.cargo/config.toml`. Инструкция по отдельной проверке контракта клиента находится в [UPSTREAM.md](UPSTREAM.md).

Для диагностики доступны:

- `scripts/diagnose_csqtt_server.sh` — отчёт о службе, процессах, базе, TUN, маршрутах и firewall;
- `scripts/csqtt-network-report.sh` — сетевой отчёт и режим UDP capture;
- `scripts/csqtt_e2e_echo.py` — UDP echo helper для отдельных сетевых тестов.

Диагностические скрипты запускаются вручную на целевом сервере. Они используют уже доступные системные утилиты.

## Структура

```text
rust-server/   оригинальный сервер 2.5.0, Cargo.lock и сборочные скрипты
shared/        используемые сервером DTLS и фрагментация пакетов
scripts/       диагностика, UDP echo helper и server asset provenance
tests/         контракт с upstream-клиентом 2.5.0
.github/       серверный CI, Docker workflows и actionlint
app/src/main/assets/domains.txt   список доменов для серверной панели
Dockerfile     сборка и запуск standalone Linux-контейнера
```

## Лицензия

Условия распространения приведены в [LICENSE](LICENSE).
