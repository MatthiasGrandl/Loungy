use wasmtime::component::bindgen;

bindgen!({
    world: "loungy",
    async: true,
});
