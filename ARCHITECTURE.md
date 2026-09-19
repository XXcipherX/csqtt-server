<!-- SPDX-FileCopyrightText: 2026 amurcanov -->
<!-- SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0 -->

# Архитектура сервера

CSQTT собирается в один Rust-бинарник. Один процесс запускает сетевой протокол, пакетный dataplane и встроенную веб-панель. Контейнеры для каждой поддерживаемой платформы собираются под её native Linux target с Rust 1.97.1 и edition 2024.

## Dataplane

Пакетный dataplane работает в отдельном потоке с Tokio `current_thread` runtime. UDP- и TUN-ввод-вывод используют readiness-модель Linux (`AsyncFd`, `try_io`), `recvmmsg` с `MSG_DONTWAIT | MSG_WAITFORONE` для приёма и `sendmmsg` с `MSG_DONTWAIT` для отправки.

Буферы находятся в заранее выделенных пулах. Горячий путь не выполняет динамических аллокаций: `tokio_io.rs` отвечает за ввод-вывод, а протокольные решения находятся в `protocol.rs` за интерфейсами `PacketSink` и `DataplaneLogic`.

Основной цикл использует biased `tokio::select!`. Команды передаются через ограниченный `tokio::sync::mpsc`, а сетевые события поступают через readiness notifications без периодического polling. Ошибка dataplane передаётся в основной runtime и приводит к упорядоченному завершению процесса, чтобы внешний supervisor мог его перезапустить.

## Веб-панель

Веб-панель входит в тот же бинарник и запускается вместе с протоколом. HTML, PWA manifest и статические ресурсы встроены в `web_panel.rs`. Панель предоставляет управление клиентами, статистику, журналы, настройки DNS и local proxy profiles.

HTTP и HTTPS принимаются на одном настраиваемом порту, по умолчанию `46002`. TLS-сертификат и ключ хранятся вместе с остальным постоянным состоянием сервера.

## Контейнер

Docker image запускает `/usr/local/bin/csqtt`. Поддерживаемый установщик использует host networking, подключает `/dev/net/tun`, предоставляет минимально необходимые capabilities и сохраняет `/etc/csqtt` в постоянном каталоге на VPS.
