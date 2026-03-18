# HTTP Contracts

## GET /api/v1/profiles

- Success: `200`
- Response:
  - `profiles`: `string[]`
- Ordering:
  - 按 `profile_id` 升序返回。

## POST /api/v1/profiles

- Success: `201`
- Request body:
  - `profile_id`: `string`
- Response body:
  - `profile_id`: `string`
- Validation:
  - 服务端先 `trim`
  - 空值返回 `400 invalid_request`
  - 精确重名返回 `409 profile_exists`
