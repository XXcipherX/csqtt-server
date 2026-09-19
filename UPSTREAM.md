# Происхождение и совместимость

Серверный код `csqtt-server` происходит из проекта [amurcanov/csqtt](https://github.com/amurcanov/csqtt) и распространяется с сохранением исходной лицензии и атрибуции.

В этот репозиторий входят:

- сервер из `rust-server/`;
- используемые сервером модули протокола из `shared/`;
- Docker image и серверные GitHub Actions workflows;
- независимые тесты совместимости Android- и iOS-клиентов.

Android-приложение, `rust-client`, Gradle-проекты и клиентские release assets сюда не входят.

## Синхронизация с клиентами

Изменения серверного протокола принимаются только после проверки:

1. значения `WIRE_PROTOCOL_REVISION` и control messages;
2. CQF1 framing, stream repair и RTP AEAD vectors;
3. контрактов с актуальным `main` [amurcanov/csqtt](https://github.com/amurcanov/csqtt);
4. контрактов с актуальным `main` [anton48/vk-turn-proxy-ios](https://github.com/anton48/vk-turn-proxy-ios);
5. Docker image contract, используемого [XXcipherX/vkturn-vps-setup](https://github.com/XXcipherX/vkturn-vps-setup).

Совместимость является проверяемым протокольным контрактом. Несовместимые upstream-изменения не переносятся автоматически.
