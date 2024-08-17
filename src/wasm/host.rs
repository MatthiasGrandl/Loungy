use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;

use super::bindings::loungy::command::host;
use super::bindings::Loungy;
use crate::paths::paths;
use crate::platform::{
    autofill, close_and_paste, close_and_paste_file, get_application_data, get_application_files,
    get_application_folders, get_frontmost_application_data, ocr,
};
use crate::window::Window;
use async_trait::async_trait;
use futures::channel::mpsc::{self, UnboundedSender};
use futures::channel::oneshot;
use futures::future::{BoxFuture, LocalBoxFuture, Shared};
use futures::{FutureExt, StreamExt};
use gpui::*;
use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::{DirPerms, FilePerms, ResourceTable, WasiCtx, WasiCtxBuilder, WasiView};

pub struct WasmHost {
    engine: Engine,
    main_thread_sender: UnboundedSender<MainThreadCall>,
    _main_thread_task: Task<()>,
}

pub struct WasmState {
    ctx: WasiCtx,
    table: ResourceTable,
    host: Arc<WasmHost>,
}

pub struct WasmExtension {
    metadata: host::Metadata,
    sender: UnboundedSender<ExtensionCall>,
}

#[derive(Clone)]
pub struct WasmExtensions {
    tasks: Vec<WasmExtensionTask>,
}

impl Global for WasmExtensions {}

impl WasmExtensions {
    pub fn list(&self) -> Vec<Arc<WasmExtension>> {
        self.tasks
            .iter()
            .filter_map(|task| {
                task.clone()
                    .now_or_never()
                    .map(|result| result.ok())
                    .flatten()
            })
            .collect()
    }
    pub async fn list_async(&self) -> Vec<Arc<WasmExtension>> {
        let mut extensions = vec![];
        for task in self.tasks.iter() {
            if let Ok(extension) = task.clone().await {
                extensions.push(extension);
            }
        }
        extensions
    }
    pub async fn list_metadata_async(&self) -> Vec<host::Metadata> {
        self.list_async()
            .await
            .into_iter()
            .map(|extension| host::Metadata {
                id: extension.metadata.id.clone(),
                title: extension.metadata.title.clone(),
                subtitle: extension.metadata.subtitle.clone(),
                icon: extension.metadata.icon.clone(),
                keywords: extension.metadata.keywords.clone(),
            })
            .collect()
    }
    pub async fn find_async(&self, id: impl ToString) -> Option<Arc<WasmExtension>> {
        let name = id.to_string();
        for task in self.tasks.iter() {
            if let Ok(extension) = task.clone().await {
                if extension.metadata.id == name {
                    return Some(extension);
                }
            }
        }
        None
    }
    pub async fn run(&self, id: impl ToString) -> Result<()> {
        let Some(extension) = self.find_async(id).await else {
            return Err(anyhow::anyhow!("Extension not found"));
        };
        Ok(extension
            .call(|instance, store| {
                async { instance.loungy_command_command().call_run(store).await }.boxed()
            })
            .await?)
    }
}

impl WasmExtension {
    pub async fn call<T, Fn>(&self, f: Fn) -> T
    where
        T: 'static + Send,
        Fn: 'static
            + Send
            + for<'a> FnOnce(&'a mut Loungy, &'a mut Store<WasmState>) -> BoxFuture<'a, T>,
    {
        let (return_tx, return_rx) = oneshot::channel();
        self.sender
            .clone()
            .unbounded_send(Box::new(move |extension, store| {
                async {
                    let result = f(extension, store).await;
                    return_tx.send(result).ok();
                }
                .boxed()
            }))
            .expect("wasm extension channel should not be closed yet");
        return_rx.await.expect("wasm extension channel")
    }
}

impl WasiView for WasmState {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.ctx
    }
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}

type MainThreadCall =
    Box<dyn Send + for<'a> FnOnce(&'a mut AsyncWindowContext) -> LocalBoxFuture<'a, ()>>;

type ExtensionCall =
    Box<dyn Send + for<'a> FnOnce(&'a mut Loungy, &'a mut Store<WasmState>) -> BoxFuture<'a, ()>>;

type WasmExtensionTask = Shared<Task<Result<Arc<WasmExtension>, Arc<anyhow::Error>>>>;

impl WasmHost {
    pub fn new(cx: &mut WindowContext) -> Arc<Self> {
        let (main_thread_sender, mut rx) = mpsc::unbounded::<MainThreadCall>();
        let main_thread_task = cx.spawn(|mut cx| async move {
            while let Some(message) = rx.next().await {
                message(&mut cx).await;
            }
        });
        let engine =
            Engine::new(Config::new().async_support(true).wasm_component_model(true)).unwrap();

        let this = Arc::new(Self {
            engine,
            main_thread_sender,
            _main_thread_task: main_thread_task,
        });

        let extensions = WasmExtensions {
            tasks: this.clone().load_extensions(cx),
        };
        let _ = extensions.list();
        cx.set_global::<WasmExtensions>(extensions);

        this
    }
    fn build_wasi_ctx(&self) -> WasiCtx {
        let mut ctx_builder = WasiCtxBuilder::new();
        ctx_builder
            .preopened_dir("/", "/", DirPerms::all(), FilePerms::all())
            .unwrap();
        ctx_builder.build()
    }
    pub fn load_extension(
        self: Arc<Self>,
        path: PathBuf,
        executor: BackgroundExecutor,
    ) -> WasmExtensionTask {
        let this = self.clone();
        let exec2 = executor.clone();
        let task = executor.spawn(async {
            let path_err = path.clone();
            async move {
                let comp = Component::from_file(&this.engine, path)?;
                let comp = Arc::new(comp);
                let state = WasmState {
                    ctx: this.build_wasi_ctx(),
                    table: ResourceTable::new(),
                    host: this.clone(),
                };
                let mut store = wasmtime::Store::new(&this.engine, state);
                let mut linker = Linker::new(&this.engine);
                host::add_to_linker(&mut linker, |state: &mut WasmState| state)?;
                wasmtime_wasi::add_to_linker_async(&mut linker)?;
                let mut instance = Loungy::instantiate_async(&mut store, &comp, &linker).await?;
                let meta = instance
                    .loungy_command_command()
                    .call_init(&mut store)
                    .await?;
                let meta = host::Metadata {
                    id: meta.id.to_string(),
                    title: meta.title,
                    subtitle: meta.subtitle,
                    icon: meta.icon,
                    keywords: meta.keywords,
                };
                let (sender, mut receiver) = mpsc::unbounded::<ExtensionCall>();
                exec2
                    .spawn(async move {
                        while let Some(call) = receiver.next().await {
                            (call)(&mut instance, &mut store).await;
                        }
                    })
                    .detach();
                Ok(Arc::new(WasmExtension {
                    metadata: meta,
                    sender,
                }))
            }
            .await
            .map_err(move |err| {
                log::error!("Failed to load extension {:?}: {:?}", path_err, err);
                Arc::new(err)
            })
        });
        task.shared()
    }
    pub fn load_extensions(self: Arc<Self>, cx: &mut AppContext) -> Vec<WasmExtensionTask> {
        let executor = cx.background_executor();
        let this = self.clone();
        let command_dir = paths().config.join("commands");
        match command_dir.read_dir() {
            Ok(command_dir) => command_dir
                .filter_map(move |entry| {
                    let entry = entry.ok()?;
                    let path = entry.path();
                    if !path.is_file() || !path.extension().is_some_and(|ext| ext.eq("wasm")) {
                        return None;
                    }
                    Some(this.clone().load_extension(path, executor.clone()))
                })
                .collect(),
            Err(err) => {
                log::error!(
                    "Failed to read command directory {:?}: {:?}",
                    command_dir,
                    err
                );
                vec![]
            }
        }
    }
}

impl WasmHost {
    fn on_main_thread<T, Fn>(&self, f: Fn) -> impl 'static + Future<Output = T>
    where
        T: 'static + Send,
        Fn: 'static + Send + for<'a> FnOnce(&'a mut AsyncWindowContext) -> LocalBoxFuture<'a, T>,
    {
        let (return_tx, return_rx) = oneshot::channel();
        self.main_thread_sender
            .clone()
            .unbounded_send(Box::new(move |cx| {
                async {
                    let result = f(cx).await;
                    return_tx.send(result).ok();
                }
                .boxed_local()
            }))
            .expect("main thread message channel should not be closed yet");
        async move { return_rx.await.expect("main thread message channel") }
    }
}

#[async_trait]
impl host::Host for WasmState {
    async fn is_open(&mut self) -> bool {
        self.host
            .on_main_thread(|cx| async { Window::is_open(cx) }.boxed_local())
            .await
    }
    async fn open(&mut self) {
        let _ = self
            .host
            .on_main_thread(|cx| async { cx.update(Window::open) }.boxed_local())
            .await;
    }
    async fn close(&mut self) {
        let _ = self
            .host
            .on_main_thread(|cx| async { cx.update(Window::close) }.boxed_local())
            .await;
    }
    async fn toggle(&mut self) {
        let _ = self
            .host
            .on_main_thread(|cx| async { cx.update(Window::toggle) }.boxed_local())
            .await;
    }
    async fn get_commands(&mut self) -> Vec<host::Metadata> {
        self.host
            .on_main_thread(|cx| {
                async {
                    let Ok(ext) =
                        cx.read_global::<WasmExtensions, WasmExtensions>(|this, _cx| this.clone())
                    else {
                        return vec![];
                    };
                    ext.list_metadata_async().await
                }
                .boxed_local()
            })
            .await
    }
    async fn run_command(&mut self, id: String) {
        let _ = self
            .host
            .on_main_thread(|cx| {
                async {
                    let Ok(ext) =
                        cx.read_global::<WasmExtensions, WasmExtensions>(|this, _cx| this.clone())
                    else {
                        return;
                    };
                    let _ = ext.run(id).await;
                }
                .boxed_local()
            })
            .await;
    }
    async fn get_app_data(&mut self, path: host::Path) -> Option<host::AppData> {
        get_application_data(&PathBuf::from(path))
    }
}
