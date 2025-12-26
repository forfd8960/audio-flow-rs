//! 应用生命周期管理模块
//!
//! 处理应用启动、退出、窗口事件等生命周期相关功能

use tauri::{AppHandle, Manager, Runtime};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::path::PathBuf;

/// 应用状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppLifecycleState {
    /// 启动中
    Starting,
    /// 运行中
    Running,
    /// 暂停中
    Paused,
    /// 退出中
    Quitting,
}

/// 生命周期事件类型
#[derive(Debug, Clone)]
pub enum LifecycleEvent {
    /// 应用启动完成
    Started,
    /// 窗口创建
    WindowCreated { label: String },
    /// 窗口销毁
    WindowDestroyed { label: String },
    /// 应用退出请求
    QuitRequested,
    /// 应用实际退出
    Exited { exit_code: i32 },
}

/// 生命周期回调类型
pub type LifecycleCallback = Box<dyn Fn(LifecycleEvent) + Send + Sync>;

/// 应用生命周期管理器
pub struct LifecycleManager {
    /// 当前状态
    state: Arc<AtomicBool>,
    /// 退出请求
    exit_requested: Arc<AtomicBool>,
    /// 回调函数
    callbacks: Arc<parking_lot::Mutex<Vec<LifecycleCallback>>>,
    /// 是否已初始化
    initialized: Arc<AtomicBool>,
}

impl Default for LifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}

impl LifecycleManager {
    /// 创建新的生命周期管理器
    pub fn new() -> Self {
        Self {
            state: Arc::new(AtomicBool::new(false)),
            exit_requested: Arc::new(AtomicBool::new(false)),
            callbacks: Arc::new(parking_lot::Mutex::new(Vec::new())),
            initialized: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 注册生命周期回调
    pub fn on_event<F>(&self, callback: F)
    where
        F: Fn(LifecycleEvent) + Send + Sync + 'static,
    {
        let mut callbacks = self.callbacks.lock();
        callbacks.push(Box::new(callback));
    }

    /// 触发生命周期事件
    pub fn emit(&self, event: LifecycleEvent) {
        let callbacks = self.callbacks.lock();
        for callback in callbacks.iter() {
            callback(event.clone());
        }
    }

    /// 设置应用已启动
    pub fn set_started(&self) {
        self.state.store(true, Ordering::SeqCst);
        self.emit(LifecycleEvent::Started);
    }

    /// 检查应用是否正在运行
    pub fn is_running(&self) -> bool {
        self.state.load(Ordering::SeqCst)
    }

    /// 请求退出应用
    pub fn request_exit(&self) {
        self.exit_requested.store(true, Ordering::SeqCst);
        self.emit(LifecycleEvent::QuitRequested);
    }

    /// 检查是否请求退出
    pub fn is_exit_requested(&self) -> bool {
        self.exit_requested.load(Ordering::SeqCst)
    }

    /// 检查是否已初始化
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::SeqCst)
    }

    /// 标记为已初始化
    pub fn set_initialized(&self) {
        self.initialized.store(true, Ordering::SeqCst);
    }
}

/// 应用初始化配置
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// 数据目录
    pub data_dir: PathBuf,
    /// 配置目录
    pub config_dir: PathBuf,
    /// 日志目录
    pub log_dir: PathBuf,
    /// 是否启用调试模式
    pub debug: bool,
    /// 默认语言
    pub language: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("audio-flow");

        Self {
            data_dir: config_dir.join("data"),
            config_dir: config_dir.clone(),
            log_dir: config_dir.join("logs"),
            debug: cfg!(debug_assertions),
            language: "zh-CN".to_string(),
        }
    }
}

impl AppConfig {
    /// 确保所有必要目录存在
    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        if !self.config_dir.exists() {
            std::fs::create_dir_all(&self.config_dir)?;
        }
        if !self.data_dir.exists() {
            std::fs::create_dir_all(&self.data_dir)?;
        }
        if !self.log_dir.exists() {
            std::fs::create_dir_all(&self.log_dir)?;
        }
        Ok(())
    }
}

/// 资源管理器
///
/// 管理应用资源的清理和释放
pub struct ResourceManager {
    /// 清理任务
    cleanup_tasks: Arc<parking_lot::Mutex<Vec<Box<dyn Fn() + Send + Sync>>>>,
}

impl Default for ResourceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceManager {
    /// 创建新的资源管理器
    pub fn new() -> Self {
        Self {
            cleanup_tasks: Arc::new(parking_lot::Mutex::new(Vec::new())),
        }
    }

    /// 注册清理任务
    pub fn register_cleanup<F>(&self, task: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        let mut tasks = self.cleanup_tasks.lock();
        tasks.push(Box::new(task));
    }

    /// 执行所有清理任务
    pub fn cleanup(&self) {
        let tasks = self.cleanup_tasks.lock();
        for task in tasks.iter() {
            task();
        }
    }
}

/// 应用统计信息
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppStats {
    /// 启动次数
    pub launch_count: u64,
    /// 总录音时长（秒）
    pub total_recording_time: u64,
    /// 总转写次数
    pub transcription_count: u64,
    /// 最后使用时间
    pub last_used: Option<i64>,
}

impl AppStats {
    /// 加载统计信息
    pub fn load(&self, config_dir: &PathBuf) -> Self {
        let stats_file = config_dir.join("stats.json");
        if stats_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&stats_file) {
                if let Ok(stats) = serde_json::from_str(&content) {
                    return stats;
                }
            }
        }
        Self::default()
    }

    /// 保存统计信息
    pub fn save(&self, config_dir: &PathBuf) -> std::io::Result<()> {
        let stats_file = config_dir.join("stats.json");
        let content = serde_json::to_string(self)?;
        std::fs::write(&stats_file, content)
    }

    /// 记录一次启动
    pub fn record_launch(&mut self) {
        self.launch_count += 1;
        self.last_used = Some(chrono::Utc::now().timestamp());
    }

    /// 记录录音时长
    pub fn record_recording_time(&mut self, seconds: u64) {
        self.total_recording_time += seconds;
    }

    /// 记录一次转写
    pub fn record_transcription(&mut self) {
        self.transcription_count += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lifecycle_manager_create() {
        let manager = LifecycleManager::new();
        assert!(!manager.is_running());
        assert!(!manager.is_exit_requested());
    }

    #[test]
    fn test_lifecycle_manager_callback() {
        let manager = LifecycleManager::new();
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        manager.on_event(move |event| {
            if matches!(event, LifecycleEvent::Started) {
                called_clone.store(true, Ordering::SeqCst);
            }
        });

        manager.set_started();
        assert!(called.load(Ordering::SeqCst));
    }

    #[test]
    fn test_app_config_default() {
        let config = AppConfig::default();
        assert!(config.debug || !config.debug); // Either is fine
        assert_eq!(config.language, "zh-CN");
    }

    #[test]
    fn test_app_stats_load_save() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_dir = temp_dir.path().to_path_buf();

        let mut stats = AppStats::default();
        stats.launch_count = 5;
        stats.total_recording_time = 3600;
        stats.transcription_count = 10;

        stats.save(&config_dir).unwrap();

        let loaded = AppStats::default().load(&config_dir);
        assert_eq!(loaded.launch_count, 5);
        assert_eq!(loaded.total_recording_time, 3600);
        assert_eq!(loaded.transcription_count, 10);
    }

    #[test]
    fn test_app_stats_record() {
        let mut stats = AppStats::default();
        stats.record_launch();
        assert_eq!(stats.launch_count, 1);

        stats.record_recording_time(60);
        assert_eq!(stats.total_recording_time, 60);

        stats.record_transcription();
        assert_eq!(stats.transcription_count, 1);
    }

    #[test]
    fn test_resource_manager() {
        let manager = ResourceManager::new();
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        manager.register_cleanup(move || {
            called_clone.store(true, Ordering::SeqCst);
        });

        manager.cleanup();
        assert!(called.load(Ordering::SeqCst));
    }
}
