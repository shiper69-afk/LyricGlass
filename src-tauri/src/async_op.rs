#[cfg(windows)]
pub fn block_async<T: Send>(
    op: windows::Foundation::IAsyncOperation<T>,
) -> windows::core::Result<T>
where
    windows::Foundation::IAsyncOperation<T>: windows_future::Future,
{
    use futures::executor::block_on;
    use windows_future::Future;
    block_on(op)
}

#[cfg(not(windows))]
pub fn block_async<T>(_op: ()) -> Result<T, ()> {
    Err(())
}