# Veles

Veles — локальный MCP-сервер на Rust, который даёт локальным LLM контролируемый доступ к веб-поиску и извлечению содержимого страниц через DuckDuckGo.

## Что Такое Veles

Veles предназначен для локальных моделей, запущенных на вашем компьютере. Он предоставляет небольшой набор MCP tools для поиска в интернете, загрузки страниц, извлечения читаемого текста и сбора источников для research-запросов.

Первый MVP намеренно консервативный: он не автоматизирует браузер, не требует API-ключей и использует DuckDuckGo через неофициальный HTML parsing.

## Возможности

- Локальный MCP-сервер через stdio.
- Реализация на Rust.
- Web search только через DuckDuckGo.
- Неофициальный парсинг HTML DuckDuckGo.
- Загрузка публичных HTTP/HTTPS страниц.
- Извлечение страниц в Markdown-like текст.
- Небольшой research workflow: поиск, загрузка top pages, возврат excerpts и URL.
- Глобальный лимит исходящих HTTP-запросов: 1 запрос в секунду по умолчанию.
- In-memory cache для search и fetch результатов.

## Текущие Ограничения

- Browser automation не входит в MVP.
- JavaScript-heavy страницы могут плохо извлекаться без браузера.
- DuckDuckGo parsing неофициальный и может сломаться, если DuckDuckGo изменит HTML.
- DuckDuckGo всё равно может вернуть 429, CAPTCHA или временную блокировку при автоматической нагрузке.
- Veles сам не синтезирует финальные ответы; он возвращает структурированные источники для LLM.

## Безопасные Настройки По Умолчанию

- Максимум 1 исходящий HTTP-запрос в секунду по умолчанию.
- Разрешены только URL со схемами `http://` и `https://`.
- `localhost`, локальные IP, private IP, link-local IP и неподдерживаемые схемы заблокированы по умолчанию.
- Количество redirects ограничено.
- Размер ответа ограничен.
- Содержимое веб-страниц всегда нужно считать недоверенным вводом.

## Установка

Установите Rust через rustup:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Перезапустите shell или выполните:

```bash
. "$HOME/.cargo/env"
```

## Сборка Из Исходников

Склонируйте репозиторий и соберите бинарник:

```bash
cargo build --release
```

Бинарник будет доступен здесь:

```text
target/release/veles
```

## Использование С OpenCode

Installer собирает release-бинарник и записывает MCP entry для OpenCode с `enabled: false`:

```bash
./scripts/install-opencode.sh
```

Путь к конфигу можно переопределить так: `OPENCODE_CONFIG=/path/to/opencode.json ./scripts/install-opencode.sh`.

Ручная настройка:

Добавьте Veles в конфиг OpenCode:

```jsonc
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "veles": {
      "type": "local",
      "command": ["/absolute/path/to/veles", "--stdio"],
      "enabled": false,
      "timeout": 60000,
      "environment": {
        "VELES_REQUESTS_PER_SECOND": "1",
        "VELES_CACHE_TTL_SECONDS": "3600",
        "VELES_DDG_REGION": "wt-wt",
        "VELES_SAFESEARCH": "moderate"
      }
    }
  }
}
```

Используйте абсолютный путь к `target/release/veles` или к любой установленной копии бинарника.

Поставьте `enabled` в `true`, когда хотите, чтобы OpenCode запускал Veles.

## Использование С LM Studio

MCP-конфигурация LM Studio может отличаться в зависимости от версии. В MCP/server settings добавьте local stdio server и укажите путь к собранному бинарнику:

```jsonc
{
  "mcpServers": {
    "veles": {
      "command": "/absolute/path/to/veles",
      "args": ["--stdio"],
      "env": {
        "VELES_REQUESTS_PER_SECOND": "1",
        "VELES_CACHE_TTL_SECONDS": "3600"
      }
    }
  }
}
```

Если ваша версия LM Studio использует другой интерфейс для регистрации MCP, выберите local stdio server и используйте ту же command/args конфигурацию.

Рекомендуемые настройки LM Studio:

- Server name: `veles`.
- Command: абсолютный путь к `target/release/veles`.
- Arguments: `--stdio`.
- Environment: оставьте `VELES_REQUESTS_PER_SECOND=1`, если специально не хотите поднять глобальный rate limit.

## Конфигурация

Veles настраивается через environment variables:

| Variable | Default | Description |
| --- | --- | --- |
| `VELES_REQUESTS_PER_SECOND` | `1` | Глобальная частота исходящих HTTP-запросов. |
| `VELES_CACHE_TTL_SECONDS` | `3600` | TTL in-memory cache. |
| `VELES_REQUEST_TIMEOUT_MS` | `15000` | Timeout HTTP-запроса. |
| `VELES_MAX_PAGE_BYTES` | `2000000` | Максимальный размер ответа. |
| `VELES_DDG_REGION` | `wt-wt` | Region parameter для DuckDuckGo. |
| `VELES_SAFESEARCH` | `moderate` | `strict`, `moderate` или `off`. |
| `VELES_USER_AGENT` | `Veles/0.1 local MCP server` | HTTP user agent. |

## Доступные MCP Tools

`current_datetime`

Возвращает текущие локальные и UTC дату/время из системных данных без использования интернета.

`web_search`

Ищет через DuckDuckGo и возвращает result titles, URLs, snippets и warnings. Если DuckDuckGo не возвращает распознаваемые результаты, Veles повторяет запрос через DuckDuckGo Lite, а затем возвращает пустой список с warnings вместо MCP error.

`web_fetch`

Загружает публичную HTTP/HTTPS страницу и возвращает текст вместе с response metadata. HTTP-статусы вроде 403, 404 и 429 возвращаются как `ok: false` со структурированной ошибкой, а не как MCP error.

`web_extract`

Загружает страницу и извлекает читаемый Markdown-like текст и metadata. Неуспешные HTTP-статусы отмечаются как `ok: false`.

`web_read`

Читает публичную HTTP/HTTPS страницу и возвращает более чистый LLM-friendly markdown с metadata, ссылками и флагом truncation. Это high-level инструмент чтения страниц, наиболее близкий к встроенному web fetcher. Неуспешные HTTP-статусы отмечаются как `ok: false`.

`web_research`

Запускает небольшой workflow: DuckDuckGo search, загрузка top results, извлечение текста и возврат excerpts источников. Ошибки отдельных источников включаются в результат и не ломают весь tool call.

## Заметки Про DuckDuckGo

Veles использует неофициальный HTML parsing DuckDuckGo. Для этого не нужен API-ключ, но это не стабильный публичный API. Такой подход подходит для локального личного использования, но не для high-volume production search.

Если DuckDuckGo изменит HTML или заблокирует автоматический трафик, `web_search` может вернуть пустой список с warnings. Veles сначала пробует DuckDuckGo HTML search, затем DuckDuckGo Lite.

## Заметки По Безопасности

Web content является недоверенным вводом. Загруженная страница может содержать prompt injection текст, который пытается повлиять на модель. Считайте возвращённый текст страницы данными, а не инструкциями.

Veles по умолчанию блокирует очевидные local/private targets, но это не полноценная sandbox. Не открывайте Veles для недоверенных удалённых пользователей.

## Лицензия

Veles распространяется под лицензией MIT.

## Roadmap

- Улучшить устойчивость DuckDuckGo parsing к изменениям HTML и временным блокировкам.
- Добавить Firefox/browser backend для JavaScript-heavy pages.
- Добавить более надёжное readable-content extraction.
- Добавить package/install scripts для других MCP clients.
