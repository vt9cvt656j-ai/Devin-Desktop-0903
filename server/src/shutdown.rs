//! 收到 SIGTERM 之后**把手里的活干完再走**。
//!
//! ## 它要修的东西
//!
//! 整个进程原来没有任何优雅退出：`axum::serve(listener, app).await?` 后面什么都没有，
//! 全仓也搜不到一处 `tokio::signal`。docker compose 停容器时发 SIGTERM，Rust 的默认处置
//! 是**直接终止进程**。
//!
//! 代价落在钱上。流式对话的计费不在 handler 里，而在它 spawn 出去的泵任务里
//! （models.rs 那句注释写得很清楚：「the request is not settled until this task finishes
//! billing」）。SIGTERM 一到，那些泵任务连同它们还没执行的 `bill(...)` 一起被就地杀掉：
//! 上游的 token 已经烧掉、运营方账单上已经记了，而 model_usage 里一行都没有，用户额度
//! 分文未动。这些请求也进不了补扣队列——把它们放进去的那段代码本身就在被杀的任务里。
//!
//! ## 两段等待，缺一不可
//!
//! 1. `with_graceful_shutdown`：停止接新连接，等在途的 HTTP 连接自然结束。
//! 2. `drain`：**再等结算任务**。这一段是必须单独有的，因为泵任务是 `tokio::spawn` 出去的，
//!    不挂在任何连接上——服务器优雅关掉之后它们照样在跑，而进程一 return 就全没了。
//!
//! 两段都有上限：一个长流可以跑好几分钟，不可能无限等，到点就走。即便到点也严格优于
//! 原来的行为——原来是一秒都不等。

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

/// 还没结算完的请求数。
static SETTLING: AtomicUsize = AtomicUsize::new(0);

/// 拿着它就代表「这一笔还没结完」。丢掉即减一，所以 panic / 提前 return 也不会漏减。
pub struct SettleGuard;

impl SettleGuard {
    pub fn new() -> Self {
        SETTLING.fetch_add(1, Ordering::SeqCst);
        Self
    }
}

impl Drop for SettleGuard {
    fn drop(&mut self) {
        SETTLING.fetch_sub(1, Ordering::SeqCst);
    }
}

pub fn settling() -> usize {
    SETTLING.load(Ordering::SeqCst)
}

/// 等一个退出信号。
///
/// SIGTERM 是 docker/compose 停容器时发的那个，ctrl_c 是本地开发时按的。两个都要认：
/// 只认 ctrl_c 的话，线上那条路径——也就是唯一会造成损失的那条——完全不受保护。
pub async fn signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut s) => {
                s.recv().await;
            }
            // 装不上信号处理器时不能变成「永远不退出」：那会让容器每次都被 SIGKILL，
            // 比没有优雅退出更糟。让这一路永远 pending，由 ctrl_c 那一路兜底。
            Err(err) => {
                tracing::error!(%err, "装不上 SIGTERM 处理器，优雅退出对容器停止将不生效");
                std::future::pending::<()>().await;
            }
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("收到 Ctrl-C，开始优雅退出"),
        _ = terminate => tracing::info!("收到 SIGTERM，开始优雅退出"),
    }
}

/// 等还在结算的请求做完，最多等这么久。
pub async fn drain(max: Duration) {
    let started = std::time::Instant::now();
    loop {
        let n = settling();
        if n == 0 {
            if started.elapsed() > Duration::from_millis(200) {
                tracing::info!(waited_ms = started.elapsed().as_millis(), "结算已排空，退出");
            }
            return;
        }
        if started.elapsed() >= max {
            // 到点还没完就走，但要留痕：这些请求的计费**确实丢了**，不能悄悄发生。
            tracing::error!(
                settling = n,
                waited_secs = max.as_secs(),
                "退出前仍有请求没结算完，它们的计费会丢失"
            );
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_guard_counts_up_and_down() {
        let before = settling();
        {
            let _a = SettleGuard::new();
            assert_eq!(settling(), before + 1);
            {
                let _b = SettleGuard::new();
                assert_eq!(settling(), before + 2);
            }
            assert_eq!(settling(), before + 1, "内层 guard 丢掉后没有减回去");
        }
        assert_eq!(settling(), before, "外层 guard 丢掉后没有减回去");
    }

    #[tokio::test]
    async fn drain_returns_immediately_when_nothing_is_settling() {
        let started = std::time::Instant::now();
        drain(Duration::from_secs(30)).await;
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "没有在途结算时不该空等",
        );
    }

    /// 等待必须有上限：一个长流可以跑好几分钟，不能把容器停止拖到被 SIGKILL。
    #[tokio::test]
    async fn drain_gives_up_after_the_deadline() {
        let _held = SettleGuard::new();
        let started = std::time::Instant::now();
        drain(Duration::from_millis(300)).await;
        let waited = started.elapsed();
        assert!(waited >= Duration::from_millis(250), "根本没等：{waited:?}");
        assert!(waited < Duration::from_secs(3), "超过上限还在等：{waited:?}");
    }
}
