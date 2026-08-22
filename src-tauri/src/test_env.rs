//! 测试里改进程级环境变量的**唯一**入口：一把 crate 级的锁 + 一个会自己还原的守卫。
//!
//! `std::env` 是**进程级**的，而 `cargo test` 默认把用例撒到多个线程上并行跑。这里以前
//! 有两把互不相识的 `static ENV_LOCK`：一把在 `auth.rs` 的 `mod auth_dir_tests` 里，一把
//! 在 `mcp.rs` 顶上。两边都在注释里写着「改它们的用例必须排队」，但各自只和自己排——
//! 同一个 `cargo test` 进程、同一个 crate，锁却是两把：
//!
//! * auth 的用例 `remove_var("HOME")` 之后断言 `auth_db_dir()` 走 USERPROFILE，中间 mcp
//!   的用例把 HOME 设成临时目录 → `auth_db_dir()` 回 `/tmp/…/.michael_ide`，断言当场红；
//! * 反向则是 mcp 的 `mcp_user_config()` 读不到自己刚写下的 `mcp.json`。
//!
//! 症状正是 mcp 那段注释自己描述的「每次红的不是同一条」，只不过跨了模块，局部的锁看
//! 不见。所以锁只能有一把，而且得住在两边都够得着的地方——crate 根。
//!
//! 改环境变量一律走 [`EnvGuard`]：构造时先加锁、再记下旧值、最后应用改动；Drop 时**先
//! 还原、再放锁**，包括断言失败 panic 出去的那条路，脏环境不会留给后面的用例。

/// 全 crate 唯一的一把环境变量锁。
///
/// `src-tauri/src` 下不允许出现第二个 `static ENV_LOCK`，也不允许在这个文件之外调用
/// `env::set_var` / `env::remove_var`——有元测试（`test/tauri-env-lock.test.mjs`）盯着，
/// 因为「又各写各的锁」是这个 bug 唯一的复发方式，而它复发时是偶发红，不会有人发现。
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// 拿着 [`ENV_LOCK`]、并在 Drop 时把碰过的变量还原回构造前状态的守卫。
///
/// 一条用例只拿**一个**：`std::sync::Mutex` 不可重入，同一个线程再拿一次就是死锁。用例
/// 中途要再改一个变量，用 [`EnvGuard::put`]，别新建第二个守卫。
pub(crate) struct EnvGuard {
    /// 碰过的键 → 构造这个守卫时它的旧值（`None` = 当时就没设）。
    saved: Vec<(String, Option<String>)>,
    /// 字段的析构排在 `Drop::drop` 函数体之后，所以还原一定发生在放锁之前。
    _serial: std::sync::MutexGuard<'static, ()>,
}

impl EnvGuard {
    /// 只排队，不改任何变量：给「断言读的是进程级环境变量」的用例用。
    pub(crate) fn serial() -> Self {
        Self {
            saved: Vec::new(),
            _serial: ENV_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner()),
        }
    }

    /// 排队 + 记下旧值 + 应用改动。`None` 表示把这个变量删掉。
    pub(crate) fn set(vars: &[(&str, Option<&str>)]) -> Self {
        let mut guard = Self::serial();
        for (key, value) in vars {
            guard.put(key, *value);
        }
        guard
    }

    /// 在同一把锁里再改一次。旧值只记**第一次**碰到这个键时的那份，所以 Drop 还原到的
    /// 始终是用例开始前的状态，而不是上一次改动前的。
    pub(crate) fn put(&mut self, key: &str, value: Option<&str>) {
        if !self.saved.iter().any(|(saved_key, _)| saved_key == key) {
            self.saved.push((key.to_string(), std::env::var(key).ok()));
        }
        Self::write(key, value);
    }

    /// 全进程写环境变量的唯一一处，且调用它时一定握着 `ENV_LOCK`。
    fn write(key: &str, value: Option<&str>) {
        unsafe {
            match value {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, old) in std::mem::take(&mut self.saved) {
            Self::write(&key, old.as_deref());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::EnvGuard;

    // 每条用例用自己的键，读也一律在锁里做：这几条用例之间不能自己再赛一次跑。
    const A: &str = "MICHAEL_IDE_ENV_GUARD_PROBE_A";
    const B: &str = "MICHAEL_IDE_ENV_GUARD_PROBE_B";
    const C: &str = "MICHAEL_IDE_ENV_GUARD_PROBE_C";
    const D: &str = "MICHAEL_IDE_ENV_GUARD_PROBE_D";

    #[test]
    fn the_guard_restores_what_was_there_before_the_case_not_before_the_last_write() {
        {
            let mut env = EnvGuard::set(&[(A, Some("first")), (B, None)]);
            assert_eq!(std::env::var(A).unwrap(), "first");
            // 同一把锁里再改一次：旧值（这里是「没设」）不能被 "first" 顶掉。
            env.put(A, Some("second"));
            assert_eq!(std::env::var(A).unwrap(), "second");
            env.put(B, Some("x"));
        }
        let _serial = EnvGuard::serial();
        assert!(std::env::var(A).is_err(), "还原到了中间态，脏环境会漏给后面的用例");
        assert!(std::env::var(B).is_err());
    }

    #[test]
    fn a_key_that_already_had_a_value_gets_that_value_back_not_deleted() {
        // 先把 C 做成「本来就有值」的样子，再让第二个守卫改它——auth 的用例改 HOME 时
        // 走的正是这条路：HOME 本来有值，还原必须还原成那个值而不是把它删掉。
        let mut outer = EnvGuard::set(&[(C, Some("ambient"))]);
        outer.put(C, Some("borrowed"));
        assert_eq!(std::env::var(C).unwrap(), "borrowed");
        drop(outer);

        let _serial = EnvGuard::serial();
        assert!(std::env::var(C).is_err(), "外层守卫记的旧值是「没设」，该删回去");
    }

    #[test]
    fn a_panicking_case_still_hands_back_a_clean_environment() {
        // panic 会把 ENV_LOCK 毒掉，这正是各处 `unwrap_or_else(|p| p.into_inner())` 存在的
        // 理由；顺带确认毒掉之后还拿得到锁。
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let panicked = std::panic::catch_unwind(|| {
            let _env = EnvGuard::set(&[(D, Some("dirty"))]);
            panic!("断言失败的那条路");
        });
        std::panic::set_hook(hook);
        assert!(panicked.is_err());

        let _serial = EnvGuard::serial();
        assert!(
            std::env::var(D).is_err(),
            "一条 panic 出去的用例把脏环境留给了后面所有用例",
        );
    }
}
