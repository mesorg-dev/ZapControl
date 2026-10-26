# ZapControl (zapret-gui)

Минималистичный чёрный GUI поверх `winws` (zapret) — форк идей [Flowseal/zapret-discord-youtube](https://github.com/Flowseal/zapret-discord-youtube) с удобным окном вместо консольных `.bat`.

## Возможности

- Запуск/остановка обхода одной кнопкой, выбор из 22 стратегий
- **Автотест**: перебирает стратегии, проверяет домены из `utils/targets.txt` (HTTP/TLS 1.2/1.3 + ping) и выбирает лучшую под твой провайдер
- **Автозапуск**: трей (HKCU Run) или служба Windows (`sc create`, start=auto)
- **Roblox-фикс картинок**: тянет JSON с актуальными IP из твоего GitHub-репо и прописывает их в `C:\Windows\System32\drivers\etc\hosts` (блок между маркерами `# ZAPRET-GUI ROBLOX BEGIN/END`)
- Служба, ipset-режимы (loaded/none/any), game filter — те же переключатели, что в оригинальном `service.bat`, только в UI

## Структура

```
zapret-gui/
├── src/                  # фронтенд (vanilla HTML/CSS/JS, без сборщика)
├── src-tauri/            # Rust-бэкенд (Tauri 2)
│   └── src/
│       ├── engine.rs     # запуск winws, служба
│       ├── tester.rs     # автотест стратегий
│       ├── roblox.rs     # hosts-фикс из JSON
│       ├── autostart.rs  # Run-ключ реестра
│       └── ...
├── strategies/           # аргументы winws (шаблоны, конвертированы из .bat)
├── lists/                # hostlists (list-general, list-google, …)
├── utils/targets.txt     # домены для автотеста
├── data/                 # пример JSON для Roblox-фикса
└── bin/                  # СЮДА положи winws.exe + WinDivert + *.bin (см. ниже)
```

## Сборка (Windows)

Требования:
- Windows 10/11
- [Rust](https://rustup.rs/) (`x86_64-pc-windows-msvc`)
- [Node.js LTS](https://nodejs.org/)
- Visual Studio Build Tools (MSVC) или полный VS

```powershell
git clone <твой-репозиторий>
cd zapret-gui
npm install
npm run build        # -> src-tauri/target/release/bundle/nsis/
```

Для разработки: `npm run dev`.

### Раскладка для запуска (portable)

Приложение ищет `bin/`, `lists/`, `strategies/` **рядом с exe**. После сборки собери папку так:

```
zapret.exe            # из target/release/
bin/                  # из релиза Flowseal: winws.exe, WinDivert*.dll/sys, *.bin
lists/                # из этого репо
strategies/           # из этого репо
utils/targets.txt
```

Быстрый способ получить `bin/`: скачай [релиз Flowseal](https://github.com/Flowseal/zapret-discord-youtube/releases), скопируй оттуда папку `bin`.

> Запуск службы и запись в hosts требуют прав администратора — GUI сам вызовет UAC.

## Roblox-фикс: свой репо с IP

Клиент скачивает JSON по URL из настроек (по умолчанию — заглушка, замени на свой):

```
https://raw.githubusercontent.com/<ты>/<репо>/main/roblox-ips.json
```

Формат — см. `data/roblox-ips.sample.json`:

```json
{
  "version": 1,
  "updated": "2026-10-27T00:00:00Z",
  "entries": [
    { "host": "tr.rbxcdn.com", "ips": ["104.18.32.47"] }
  ]
}
```

Обновлять IP удобно скриптом `scripts/update_roblox_ips.py` (резолвит домены → пишет JSON):

```bash
python3 scripts/update_roblox_ips.py -o roblox-ips.json
git add roblox-ips.json && git commit -m "update roblox ips" && git push
```

На **Android** (фаза 2) этот же JSON будет применяться не через hosts, а как DNS-ответы внутри `VpnService`.

## Автотест

Кнопка «Автотест» в GUI:
1. Останавливает текущий winws
2. По очереди запускает каждую стратегию
3. Для каждого домена из `utils/targets.txt` делает `curl -I` (HTTP/1.1, TLS1.2, TLS1.3) + ping
4. Считает OK/ERR, выбирает стратегию с максимумом OK
5. Сохраняет выбор как «последняя стратегия» (её подхватит автозапуск)

Служба `zapret` должна быть удалена перед тестом — иначе конфликт (GUI об этом напишет).

## Фаза 2: Android (через VpnService)

Важно: **полноценный nfqws без root на Android невозможен** — TUN не даёт десинхронизацию TCP payload как WinDivert/NFQUEUE. Реалистичный план:

1. Kotlin + Jetpack Compose, чёрная тема (как здесь)
2. `VpnService` — маршрутизация трафика приложения
3. Обход — урезанный (DNS-level + частичный userspace-TCP), либо root-режим (nfqws + iptables)
4. Roblox-фикс — тот же JSON, но ответы DNS отдаёт само приложение (виртуальный DNS в VPN), hosts не нужен
5. Автотест — те же URL, проверкаconnectivity через curl-эквивалент

Логика тестов и формат JSON уже вынесены так, чтобы переиспользовать targets/strategies-файлы.

## Стратегии

Конвертированы из Flowseal (`scripts/convert_strategies.py`). Чтобы обновить стратегии из апстрима:

```bash
git clone --depth 1 https://github.com/Flowseal/zapret-discord-youtube _ref/zapret-discord-youtube
python3 scripts/convert_strategies.py
```

Шаблоны в `strategies/*.txt` используют плейсхолдеры `%BIN%`, `%LISTS%`, `%GameFilterTCP%`, `%GameFilterUDP%` — их подставляет Rust-бэкенд при запуске.

## Лицензия / Attribution

Стратегии и `bin/` — из [bol-van/zapret](https://github.com/bol-van/zapret) и [Flowseal/zapret-discord-youtube](https://github.com/Flowseal/zapret-discord-youtube). Соблюдай их лицензии при распространении бинарей.
