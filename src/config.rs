use std::{fs, path::PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
struct AppConfig {
  /**
   * 最大备份数量
   * 
   * 设置为0时, 表示不备份
   */
  max_backups: i64,
  /**
   * 是否包含声笔简整的语言模型
   */
  include_octagram: bool,
  /**
   * 是否包含自定义配置
   */
  update_custom_config: bool,
  /**
   * 当前版本号
   */
  version_id: String,
  /**
   * 当前版本名称
   */
  version_name: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct RimeConfig {
  /**
   * RIME 用户配置路径
   */
  config_path: PathBuf,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
  pub working_dir: PathBuf,
  app: AppConfig,
  rime: RimeConfig,
}

impl Config {
  pub fn new(working_dir: PathBuf) -> Self {
    let config_file = working_dir.join("config.toml");
    if config_file.exists() {
      let content = fs::read_to_string(config_file).unwrap();
      let config: Config = toml::from_str(content.as_str()).unwrap();
      return config;
    } else {
      let config = Self {
        working_dir,
        app: AppConfig {
          max_backups: 1,
          include_octagram: false,
          update_custom_config: true,
          version_id: String::from("init-version"),
          version_name: String::from("20051203"),
        },
        rime: RimeConfig {
          config_path: Self::default_rime_config_path(),
        },
      };

      match config_file.parent() {
        Some(parent) => {
          if !parent.exists() {
            fs::create_dir_all(parent).unwrap();
          }
        }
        None => {}
      }

      let content = toml::to_string(&config).unwrap();
      fs::write(config_file, content).unwrap();

      return config;
    }
  }

  #[cfg(not(target_os = "windows"))]
  pub fn path_in_home(sub_path: &str) -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap()).join(sub_path)
  }

  #[cfg(target_os = "macos")]
  pub fn default_rime_config_path() -> PathBuf {
    Self::path_in_home("Library/Rime")
  }

  #[cfg(target_os = "windows")]
  pub fn path_in_home(sub_path: &str) -> PathBuf {
    PathBuf::from(std::env::var("APPDATA").unwrap()).join(sub_path)
  }

  #[cfg(target_os = "windows")]
  pub fn default_rime_config_path() -> PathBuf {
    Self::path_in_home("Rime")
  }

  pub fn get_rime_config_path(&self) -> PathBuf {
    self.rime.config_path.clone()
  }

  pub fn get_version_id(&self) -> String {
    self.app.version_id.clone()
  }

pub fn get_version_name(&self) -> String {
    self.app.version_name.clone()
  }

  pub fn set_version(&mut self, version_id: String, version_name: String) {
    self.app.version_id = version_id;
    self.app.version_name = version_name;
  }

  pub fn save(&self) {
    let content = toml::to_string(self).unwrap();
    fs::write(self.working_dir.join("config.toml"), content).unwrap();
  }

  pub fn get_max_backups(&self) -> i64 {
    self.app.max_backups
  }

  pub fn is_include_octagram(&self) -> bool {
    self.app.include_octagram
  }
}

