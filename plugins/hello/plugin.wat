(module
  (memory (export "memory") 2)
  (data (i32.const 4096) "{\22state\22:{\22http_permission\22:0},\22ui\22:{\22type\22:\22column\22,\22children\22:[{\22type\22:\22label\22,\22text\22:\22Hola desde un plugin WASM de terceros\22},{\22type\22:\22label\22,\22text\22:\22Sin red: perfiles efímeros y permisos explícitos\22}]}}")
  (func (export "folio_api_version") (result i32) i32.const 1)
  (func (export "folio_alloc") (param i32) (result i32) i32.const 1024)
  (func (export "folio_handle") (param i32 i32) (result i32) i32.const 208)
  (func (export "folio_result_ptr") (result i32) i32.const 4096)
  (func (export "folio_result_len") (result i32) i32.const 208)
)
