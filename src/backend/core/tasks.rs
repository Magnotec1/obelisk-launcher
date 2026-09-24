use std::sync::LazyLock;

/// Dedicated thread pool for background I/O operations (HTTP requests, file downloads, disk scans).
/// Having a bounded thread pool prevents unbounded OS thread spawning and resource exhaustion.
static IO_POOL: LazyLock<rayon::ThreadPool> = LazyLock::new(|| {
    rayon::ThreadPoolBuilder::new()
        .num_threads(12)
        .thread_name(|i| format!("obelisk-io-{}", i))
        .build()
        .unwrap_or_else(|e| {
            eprintln!("Warning: Failed to create custom I/O pool: {e}. Falling back to default.");
            rayon::ThreadPoolBuilder::new()
                .build()
                .expect("Failed to initialize background thread pool")
        })
});

/// Spawns an I/O or network task on the managed background thread pool.
pub fn spawn_io<F>(task: F)
where
    F: FnOnce() + Send + 'static,
{
    IO_POOL.spawn(task);
}

/// Spawns a CPU-intensive task (decompression, checksumming, heavy parsing) on the Rayon global pool.
pub fn spawn_compute<F>(task: F)
where
    F: FnOnce() + Send + 'static,
{
    rayon::spawn(task);
}

/// Spawns an I/O task and sends the returned value to the given Relm4 sender.
pub fn spawn_io_with_sender<T, F>(sender: relm4::Sender<T>, task: F)
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    IO_POOL.spawn(move || {
        let res = task();
        let _ = sender.send(res);
    });
}

/// Spawns a compute task and sends the returned value to the given Relm4 sender.
pub fn spawn_compute_with_sender<T, F>(sender: relm4::Sender<T>, task: F)
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    rayon::spawn(move || {
        let res = task();
        let _ = sender.send(res);
    });
}
