use rustyscript::{json_args, Module, Runtime, RuntimeOptions};

// extension!(
//     render,
//     esm_entry_point = "ext:src/deno/js/render.ts",
//     esm = [ dir "src/deno/js", "render.ts" ],
// );

pub fn deno_test() -> anyhow::Result<()> {
    let mut runtime = Runtime::new(RuntimeOptions {
        ..Default::default()
    })?;
    let module = Module::load("src/deno/js/render.ts")?;
    let module_handle = runtime.load_module(&module)?;
    runtime.call_entrypoint(&module_handle, json_args!())?;
    let module = Module::new(
        "Module.tsx",
        r#"
        import React, { useState, useEffect, ReactNode, ReactElement } from "https://esm.sh/react@18.2.0";
        
        const Root = (props): ReactNode => {
            return <loungy:root>{props.children}</loungy:root>;
        };

        export default (): ReactElement => {
            const [count, setCount] = useState(0);
        
            useEffect(() => {
                const timerId = setInterval(() => {
                    setCount(count + 1);
                    console.log("interval");
                }, 1000);
                console.log("timerId", timerId);
                return () => clearInterval(timerId);
            });
            return <Root>{count}</Root>;
        }
    "#,
    );
    let _ = runtime.load_module(&module)?;

    let module = Module::new(
        "init.tsx",
        r#"
        import Module from "./Module.tsx";

        export default async () => {
            return await render(<Module />);
        }
        "#,
    );

    let module_handle = runtime.load_module(&module)?;
    runtime.call_entrypoint(&module_handle, json_args!())?;

    Ok(())
}
