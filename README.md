# 声笔输入法自动更新程序

自动检测 gitee.com 上最新的 release 信息，如果本地安装的和 release 的不一致，则进行更新。
支持更新前备份原有配置。

## 快速开始

```shell
# 更新本地方案
./sbsrf-update
```

### ios 平台仓输入法更新声笔方案

1. 将手机和电脑连接到同一个WiFi下
2. 打开仓输入法的 Wi-Fi 上传方案，并让屏幕保持在该界面（不能锁屏）
3. 在电脑上运行命令更新
  ```shell
  ./sbsrf-update --platform ios --host 192.168.1.2
  ```
4. 在手机上重新部署

## CLI 参数说明：

```text
Usage: sbsrf-update [OPTIONS]

Options:
  -w, --working-dir <WORKING_DIR>  工作目录，默认在 $HOME/.sbsrf-update
  -f, --force                      强制更新，默认本地版本和服务器版本一致时不作任何操作，强制更新时即使版本相同也会更新
  -p, --platform <PLATFORM>        目标操作系统，默认为当前系统 [default: macos]
      --host <HOST>                iOS 设备 IP 地址
  -r, --restore                    还原备份的版本
  -h, --help                       Print help
  -V, --version                    Print version
```

### 配置文件

```text
# 工作目录，cli 参数 -w 指定，无须更改
working_dir = "/Users/hotleave/.sbsrf-update/macos"

[app]
# 最大备份数量，设置为 0 时不备份
max_backups = 1

# 是否下载 octagram.zip，如果使用的是整句输入方案的话，需要该文件，否则不需要
include_octagram = false

# 版本信息，更新后自动填写，无须更改
version_id = "33cc3af76d53139c4cb8463c0f4637d316caad0e"
version_name = "20240331"

[rime]
# Rime 用户目录
config_path = "/Users/hotleave/Library/Rime"
```

## 版本信息

### pending

- [新增] 支持从 github 更新

### 0.2.1

- [新增] 更新后自动部署
- [修复] Windows下备份时由于小狼毫算法服务正在运行导致无法复制文件而报错的问题

### 0.2.0

- [新增] 支持 iOS 平台设备上的仓输入法更新
- [新增] 支持还原到某个备份版本

### 0.1.0

- [新增] 支持 MacOS 及 Windows 平台声笔输入法的更新
- [新增] 支持更新前备份