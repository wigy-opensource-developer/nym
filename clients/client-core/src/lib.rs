use std::future::Future;

pub mod client;
pub mod config;
pub mod error;
pub mod init;

#[cfg(target_arch = "wasm32")]
pub(crate) fn spawn_future<F>(future: F)
where
    F: Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(future);
}

//#[cfg(not(target_arch = "wasm32"))]
//pub(crate) fn spawn_future<F>(future: F)
//where
//    F: Future + Send + 'static,
//    F::Output: Send + 'static,
//{
//    tokio::spawn(future);
//}

pub(crate) fn spawn_future<F>(task_name: &str, future: F)
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    //tokio::task::Builder::default()
    //    .name(task_name)
    //    .spawn(future);
    tokio::spawn(future);
}

//pub(crate) fn spawn_future_named<F>(future: F, task_name: &str)
//where
//    F: Future + Send + 'static,
//    F::Output: Send + 'static,
//{
//    tokio::task::Builder::default()
//        .name(task_name)
//        .spawn(future);
//    //tokio::spawn(future);
//}
