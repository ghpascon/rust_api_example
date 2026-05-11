# rust_api_example

API async em Rust com `TagList`, índice principal por `identifier` e índice auxiliar EPC -> TID para buscas rápidas.

## Endpoints

- `GET /health`
- `POST /api/tags`
- `GET /api/tags`
- `GET /api/tags/{identifier}`
- `GET /api/tags/by-epc/{epc}`
- `GET /api/tags/by-tid/{tid}`
- `DELETE /api/tags/{identifier}`
- `DELETE /api/tags/before/{timestamp_ms}`

## Exemplo de payload

```json
{
  "epc": "ABCD1234",
  "tid": "00112233445566778899AABB",
  "ant": 1,
  "rssi": -42
}
```

## Regras

- `epc`: obrigatório, hexadecimal, tamanho múltiplo de 4
- `tid`: opcional, mas se enviado deve ter 24 chars hex
- `ant`: inteiro positivo
- `rssi`: inteiro negativo entre `-255` e `-1`
- `identifier`: usa `tid` quando existir, ou `_{epc}` quando não existir
