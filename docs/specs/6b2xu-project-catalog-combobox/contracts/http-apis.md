# HTTP Contracts

## GET /api/v1/projects

- Success: `200`
- Response:
  - `projects`: `string[]`
- Ordering:
  - 按 `project_id` 升序返回。

## POST /api/v1/projects

- Success: `201`
- Request body:
  - `project_id`: `string`
- Response body:
  - `project_id`: `string`
- Validation:
  - 服务端先 `trim`
  - 空值返回 `400 invalid_request`
  - 精确重名返回 `409 project_exists`
