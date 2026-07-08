use crate::error::*;
use crate::types::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// 自动化录制器 - 记录和回放操作序列
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recording {
    /// 录制名称
    pub name: String,
    /// 操作序列
    pub commands: Vec<AutomationCommand>,
    /// 录制时间戳
    pub timestamp: i64,
    /// 元数据
    pub metadata: RecordingMetadata,
}

/// 录制元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingMetadata {
    /// 操作系统
    pub os: String,
    /// 屏幕分辨率
    pub screen_resolution: Option<(u32, u32)>,
    /// 描述
    pub description: Option<String>,
}

impl Recording {
    /// 创建新录制
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            commands: Vec::new(),
            timestamp: chrono::Utc::now().timestamp(),
            metadata: RecordingMetadata {
                os: std::env::consts::OS.to_string(),
                screen_resolution: None,
                description: None,
            },
        }
    }

    /// 添加命令
    pub fn add_command(&mut self, command: AutomationCommand) {
        self.commands.push(command);
    }

    /// 保存到文件
    pub fn save_to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    /// 从文件加载
    /// 
    /// # 安全性
    /// 只加载来自可信来源的录制文件。恶意构造的 JSON 可能包含
    /// 危险的自动化命令（如删除文件、执行任意输入等）。
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let recording: Recording = serde_json::from_str(&content)?;
        Ok(recording)
    }

    /// 获取命令数量
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

/// 回放器 - 执行录制的操作序列
pub struct Replayer {
    recording: Recording,
    current_index: usize,
}

impl Replayer {
    /// 创建回放器
    pub fn new(recording: Recording) -> Self {
        Self {
            recording,
            current_index: 0,
        }
    }

    /// 从文件加载录制
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let recording = Recording::load_from_file(path)?;
        Ok(Self::new(recording))
    }

    /// 获取下一个命令
    pub fn next_command(&mut self) -> Option<&AutomationCommand> {
        if self.current_index < self.recording.commands.len() {
            let cmd = &self.recording.commands[self.current_index];
            self.current_index += 1;
            Some(cmd)
        } else {
            None
        }
    }

    /// 重置到开始
    pub fn reset(&mut self) {
        self.current_index = 0;
    }

    /// 获取进度
    pub fn progress(&self) -> (usize, usize) {
        (self.current_index, self.recording.commands.len())
    }

    /// 是否完成
    pub fn is_finished(&self) -> bool {
        self.current_index >= self.recording.commands.len()
    }

    /// 获取录制信息
    pub fn recording(&self) -> &Recording {
        &self.recording
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recording_creation() {
        let recording = Recording::new("test");
        assert_eq!(recording.name, "test");
        assert!(recording.is_empty());
    }

    #[test]
    fn test_add_command() {
        let mut recording = Recording::new("test");
        recording.add_command(AutomationCommand::Mouse(MouseAction::Move { 
            x: 100, 
            y: 200,
            mode: CoordinateMode::Absolute 
        }));
        assert_eq!(recording.len(), 1);
    }

    #[test]
    fn test_save_and_load() {
        let mut recording = Recording::new("test");
        recording.add_command(AutomationCommand::Mouse(MouseAction::Click { 
            button: MouseButton::Left 
        }));

        let path = std::env::temp_dir().join("test_recording.json");
        recording.save_to_file(&path).unwrap();

        let loaded = Recording::load_from_file(&path).unwrap();
        assert_eq!(loaded.name, "test");
        assert_eq!(loaded.len(), 1);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_replayer() {
        let mut recording = Recording::new("test");
        recording.add_command(AutomationCommand::Mouse(MouseAction::Move { 
            x: 100, 
            y: 200,
            mode: CoordinateMode::Absolute 
        }));
        recording.add_command(AutomationCommand::Mouse(MouseAction::Click { 
            button: MouseButton::Left 
        }));

        let mut replayer = Replayer::new(recording);
        
        assert!(!replayer.is_finished());
        assert_eq!(replayer.progress(), (0, 2));

        replayer.next_command();
        assert_eq!(replayer.progress(), (1, 2));

        replayer.next_command();
        assert!(replayer.is_finished());

        replayer.reset();
        assert!(!replayer.is_finished());
        assert_eq!(replayer.progress(), (0, 2));
    }
}
