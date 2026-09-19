# csqtt-server

[![Server CI](https://github.com/XXcipherX/csqtt-server/actions/workflows/server-ci.yml/badge.svg)](https://github.com/XXcipherX/csqtt-server/actions/workflows/server-ci.yml)
[![Docker CI](https://github.com/XXcipherX/csqtt-server/actions/workflows/docker-ci.yml/badge.svg)](https://github.com/XXcipherX/csqtt-server/actions/workflows/docker-ci.yml)
[![Upstream CSQTT contract](https://github.com/XXcipherX/csqtt-server/actions/workflows/upstream-csqtt-contract.yml/badge.svg)](https://github.com/XXcipherX/csqtt-server/actions/workflows/upstream-csqtt-contract.yml)
[![iOS contract](https://github.com/XXcipherX/csqtt-server/actions/workflows/ios-contract-ci.yml/badge.svg)](https://github.com/XXcipherX/csqtt-server/actions/workflows/ios-contract-ci.yml)

Серверная часть CSQTT на Rust без Android-приложения и Gradle. Репозиторий содержит сервер, общие модули протокола, тесты совместимости и multi-platform Docker image.

Проект основан на [amurcanov/csqtt](https://github.com/amurcanov/csqtt). Дополнительная информация о происхождении кода и политике совместимости приведена в [UPSTREAM.md](UPSTREAM.md), обзор внутреннего устройства — в [ARCHITECTURE.md](ARCHITECTURE.md).

## Возможности

- туннелирование IP-пакетов через RTP AEAD и протокол `CSQTT-WIRE-3`;
- userspace TUN и высокопроизводительный пакетный dataplane;
- встроенная HTTP/HTTPS-панель для управления клиентами, настройки сервера, просмотра статистики и журналов;
- постоянное хранение конфигурации, базы клиентов и TLS-материалов;
- Docker images для `linux/amd64` и `linux/arm64`;
- автоматическая проверка совместимости с Android- и iOS-клиентами.

## Установка на VPS

Для установки и обновления используйте [XXcipherX/vkturn-vps-setup](https://github.com/XXcipherX/vkturn-vps-setup):

~~~bash
git clone https://github.com/XXcipherX/vkturn-vps-setup.git
cd vkturn-vps-setup
sudo bash csqtt-docker-setup.sh
~~~

Установщик загружает образ, проверяет версию протокола, настраивает TUN, IPv4 forwarding, NAT и firewall, затем проверяет готовность UDP listener и веб-панели. Повторный запуск обновляет контейнер с сохранением конфигурации и базы клиентов.

После установки панель доступна по адресу:

~~~text
https://SERVER_IP:46002/
~~~

Порты и учётные данные панели можно изменить во время установки. При использовании автоматически созданного TLS-сертификата браузер может показать предупреждение о недоверенном центре сертификации.

## Docker image

Публичный образ:

~~~text
ghcr.io/xxcipherx/csqtt-server:latest
~~~

Образ предназначен для `linux/amd64` и `linux/arm64`. Контейнер использует host networking и требует `/dev/net/tun`, capabilities `NET_ADMIN` и `NET_RAW`, включённый IPv4 forwarding и корректные NAT/firewall rules. Поэтому для обычной установки рекомендуется готовый установщик, а не отдельный `docker run`.

Данные сервера хранятся в `/etc/csqtt` внутри контейнера. Установщик подключает к нему постоянный каталог `/opt/csqtt-docker/data` на VPS.

### Порты по умолчанию

| Назначение | Протокол | Порт |
| --- | --- | ---: |
| CSQTT peer | UDP | `46000` |
| Веб-панель | TCP | `46002` |

При использовании host networking колонка `PORTS` в выводе `docker ps` остаётся пустой — сервер слушает порты непосредственно на хосте.

## Совместимые клиенты

Текущая версия сервера использует `CSQTT-WIRE-3` и проверяется со следующими клиентами:

- Android: [amurcanov/csqtt](https://github.com/amurcanov/csqtt);
- iOS: [anton48/vk-turn-proxy-ios](https://github.com/anton48/vk-turn-proxy-ios).

Contract workflows проверяют control plane, stream repair, CQF1 framing и RTP AEAD vectors на актуальных версиях обоих клиентских проектов.

## Автоматические проверки

- `Server CI` — форматирование, Clippy, unit/protocol tests, release build, protocol revision и `cargo audit`;
- `Docker CI` — сборка OCI images для `amd64` и native `arm64` без публикации;
- `Build and push Docker image` — публикация multi-platform image вручную или по тегу `v*`;
- `Current upstream CSQTT contract` и `Current iOS CSQTT contract` — тесты совместимости клиентов;
- `Workflow lint` и `Dependency Review` — проверка GitHub Actions и изменений зависимостей.

## Структура репозитория

~~~text
rust-server/   сервер CSQTT и Cargo.lock
shared/        общие модули framing, FEC и scheduler
tests/         независимые тесты совместимости клиентов
scripts/       диагностика сети и UDP echo helper
.github/       CI, публикация Docker image и contract workflows
~~~

## Лицензия

Код распространяется по [PolyForm Noncommercial License 1.0.0](LICENSE). Коммерческое использование требует отдельного письменного разрешения лицензиара. При распространении исходного кода и производных работ необходимо сохранять предусмотренные лицензией уведомления и атрибуцию.
