# Semantic-Machine

The Semantic Machine is an advanced natural language processing (NLP) and semantic analysis platform designed to extract, interpret, and evaluate textual data related to financial markets, with a specialized emphasis on the cryptocurrency sector.

## Project Structure

 - crates:
   - llm-bert - large language bert model runtime entrypoint
   - nats-middleware - NATS middleware for handling subject building and routing
 - apps:
   - telegram-worker - telegram worker reading data from telegram channels and using semantic strategies to analyze data and generate insights
   - api-server - RESTful API server for exposing semantic analysis endpoints
   
## Technology stack

- Rust version 1.90.0 or newer

## Dependencies

| Service             | Image / Version                                |
| ------------------- | ---------------------------------------------- |
| PostgreSQL          | `timescale/timescaledb:latest-pg17`            |
| NATS                | `nats:latest`                                  |
| Redis               | `redis:7-alpine`                               |
| MinIO               | `minio/minio:latest`                           |
| MinIO Client (init) | `minio/mc:latest`                              |
| Prometheus          | `prom/prometheus:latest`                       |
| Grafana             | `grafana/grafana:latest`                       |
| Jaeger              | `jaegertracing/all-in-one:latest`              |
| PGAdmin             | `dpage/pgadmin4:latest`                        |
| Postgres Exporter   | `prometheuscommunity/postgres-exporter:latest` |
| Redis Exporter      | `oliver006/redis_exporter:latest`              |

## License

This project is licensed under the GNU AFFERO GENERAL PUBLIC LICENSE - see the [LICENSE](LICENSE) file for details.