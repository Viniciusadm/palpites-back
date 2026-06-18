# palpites-back

Backend Rust com **arquitetura hexagonal (Ports & Adapters)**: domínio puro,
casos de uso desacoplados de I/O via traits, e adaptadores concretos
(axum + sqlx/mysql + jwt + argon2) injetados nas bordas.

> Este projeto foi gerado a partir do template `rust-backend-template`.

## Gerar um novo projeto a partir deste template

```bash
# instalar a ferramenta uma vez
cargo install cargo-generate

# a partir de um caminho local
cargo generate --path /caminho/para/rust-backend-template --name meu-servico

# ou de um repositório Git
cargo generate --git <url-do-repo> --name meu-servico
```

`--name meu-servico` define `project-name` (kebab-case) e `crate_name`
(snake_case, ex.: `meu_servico`) automaticamente.

## Rodar

```bash
cp .env.example .env          # ajuste DATABASE_URL e JWT_SECRET
cargo test                    # testes de caso de uso (não precisam de banco)
cargo run                     # sobe a API (precisa de MySQL para persistência)
```

Endpoints do slice de exemplo:

- `GET  /health`
- `POST /api/v1/auth/register`
- `POST /api/v1/auth/login`
- `GET  /api/v1/auth/me` (requer `Authorization: Bearer <token>`)

## Arquitetura

As dependências apontam **para dentro**: `api → application → domain`. A
infraestrutura implementa os traits (ports) definidos em `application`, e é
injetada nos handlers por request.

```
src/
├── domain/            Regras e tipos puros (sem axum/sqlx/serde)
│   ├── value_objects.rs   DomainId, Email, NonEmptyString, UtcDateTime
│   └── users/entities.rs  struct User
├── application/       Casos de uso + ports (traits)
│   └── auth/
│       ├── ports.rs       UserRepository, PasswordHasher, TokenIssuer, TokenVerifier
│       ├── commands.rs    DTOs de entrada/saída
│       └── use_cases.rs   AuthUseCases<R,H,T> (genérico sobre os ports)
├── infrastructure/    Adaptadores concretos (implementam os ports)
│   ├── auth/              Argon2PasswordHasher, JwtTokenService
│   └── repositories/      MySqlUserRepository + mapper (linha → domínio)
├── database/          pool + transaction
├── config/            AppConfig::from_env()
├── errors/            AppError (enum único) + códigos estáveis
└── api/               Camada HTTP (axum)
    ├── state.rs           AppState (só infra: pool + jwt_secret)
    ├── dto/               Request/Response + From para commands
    ├── extractors/        AuthenticatedUser (FromRequestParts)
    └── routes/            health + v1 (handlers + app_error)
```

Princípios que o template segue (e que você deve manter):

- **Sem estado global / service locator.** O `AppState` guarda só
  infraestrutura. Cada handler constrói seu caso de uso por request.
- **Domínio não conhece framework.** Nada de `serde`/`sqlx`/`axum` em `domain/`.
- **Erros num único `AppError`**, traduzido para HTTP em `app_error()`.
- **Casos de uso genéricos sobre os ports** → testáveis com fakes, sem banco.

## Como adicionar uma feature nova (ex.: `products`)

Replique o mesmo padrão, camada por camada:

1. **Application** — `src/application/products/`
   - `ports.rs`: trait `ProductRepository` (+ records de persistência)
   - `commands.rs`: DTOs de entrada/saída
   - `use_cases.rs`: `ProductUseCases<R>` genérico sobre o(s) port(s)
   - `mod.rs`: `pub use commands/ports/use_cases::*;`
   - registre `pub mod products;` em `src/application/mod.rs`
2. **Domain** — `src/domain/products/{mod.rs,entities.rs}`; value objects novos
   vão em `src/domain/value_objects.rs`. Sem dependências de framework.
3. **Infrastructure** — `src/infrastructure/repositories/mysql_products.rs`
   implementa o trait; conversões de linha → domínio em `mapper.rs`. Registre
   em `repositories/mod.rs`.
4. **API** — `src/api/routes/v1/products.rs` com `router()` + handlers;
   instancie o caso de uso por request e trate erros com `app_error()`. Faça
   `.merge(products::router())` em `src/api/routes/v1/mod.rs`.
5. **DTOs** — `src/api/dto/products.rs` com `From<Request> for Command`.
6. **Tests** — `tests/products_use_cases.rs` com fakes dos ports (veja
   `tests/auth_use_cases.rs` como modelo).

## Variáveis de ambiente

Veja `.env.example`. Resumo:

| Variável         | Default                | Descrição                              |
|------------------|------------------------|----------------------------------------|
| `HTTP_HOST`      | `127.0.0.1`            | Host de bind                           |
| `HTTP_PORT`      | `3001`                 | Porta                                  |
| `DATABASE_URL`   | —                      | Conexão MySQL (sem ela, sem persistência) |
| `JWT_SECRET`     | `development-only-secret` | Segredo de assinatura do JWT        |
| `RUN_MIGRATIONS` | `true`                 | Rodar migrations no boot               |
| `FILES_BACKEND`  | `local`                | Backend de arquivos: `local` ou `s3` (stub) |
| `FILES_BUCKET`   | `palpites-local`       | Bucket/diretório lógico dos arquivos   |
| `FILES_LOCAL_DIR`| `./storage`            | Raiz do adapter local                  |
| `FILES_PUBLIC_BASE_URL` | `http://127.0.0.1:3001/files-local` | Base das URLs assinadas (local) |
| `FILES_MAX_BYTE_SIZE` | `5242880`         | Tamanho máximo de upload em bytes (5 MiB) |
| `S3_REGION` / `S3_ENDPOINT` | —          | Config do backend s3 (stub neste build) |
| `JOBS_ENABLED`   | `true`                 | Liga o scheduler de jobs em background (tokio) |
| `JOBS_REMINDER_INTERVAL_SECONDS` | `300`  | Intervalo do job de lembretes de palpite |
| `JOBS_REMINDER_WINDOW_MINUTES` | `60`     | Janela para "lock se aproximando" dos lembretes |
| `JOBS_DISPATCH_INTERVAL_SECONDS` | `60`   | Intervalo do job de entrega de notificações |
