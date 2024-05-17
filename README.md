# 声笔输入法自动更新程序

自动检测 gitee.com 上最新的 release 信息，如果本地安装的和 release 的不一致，则进行更新。
支持更新前备份原有配置。

## 快速开始

```shell
# 更新本地方案，如果本地未安装输入法程序会自动安装
./sbsrf-update
```

### ios 平台仓输入法更新声笔方案

1. 将手机和电脑连接到同一个WiFi下
2. 打开仓输入法的 Wi-Fi 上传方案，并让屏幕保持在该界面（不能锁屏）
3. 在电脑上运行命令更新
  ```shell
  # 仅需要执行一次，以后更新时只执行第二条命令即可, ios 可以替换成你期望的名称
  ./sbsrf-update device add ios
  ./sbsrf-update update ios
  ```
4. 在手机上重新部署

## 工作原理

程序从声笔的发布页面获却最新的发布信息并和本地配置文件中的版本进行对比，如果不一致，就说明有了新的发布版本。
更新时根据配置文件中输入法程序决定需要下载哪些文件，这些文件都是 zip 包，下载后解压到 Rime 的用户目录，发布包中不包含用户词库，因此不会对本地的用户词库造成破坏。
更新后程序将新的版本号记录在配置文件当中，以便下次更新时使用。

为了以防万一，程序默认开启了备份功能，在每次更新前会将当前的 Rime 用户目录备份到工作目录下。当备份的数量超过配置文件中指定的数量时，会自动清理老版本的备份。

还原时，会将当前的 Rime 用户目录删除(仓输入法目前不会删除)，然后在将要还原的版本复制到该位置完成替换。

无论是更新还是还原后，程序都会尝试重新部署，使操作生效。

> Windows上用本程序对小狼毫进行更新时，需要先停掉它的算法服务，等更新完成后再启动，期间可能会造成输入法不可使用


## CLI 参数说明：

```text
声笔输入法更新程序，支持安装、更新、备份及还原，支持 iOS 设备

Usage: sbsrf-update [OPTIONS]
       sbsrf-update device [COMMAND]
       sbsrf-update update [OPTIONS] [name]
       sbsrf-update restore [OPTIONS] [name]
       sbsrf-update clean [OPTIONS]
       sbsrf-update help [COMMAND]...

Options:
  -g, --github   使用 github 上的发布信息而不是默认的 gitee
  -h, --help     Print help
  -V, --version  Print version

sbsrf-update device:
设备管理
  -h, --help  Print help

sbsrf-update update:
升级词声笔输入法词库
  -H, --host <host>  远程设备地址
  [name]         设备唯一名称 [default: macos]

sbsrf-update restore:
还原到某个备份版本
  -H, --host <host>  远程设备地址
  [name]         设备唯一名称 [default: macos]

sbsrf-update clean:
清理工作目录缓存
  -a, --all  删除整个工作目录，包含设备及备份

sbsrf-update help:
Print this message or the help of the given subcommand(s)
  [COMMAND]...  Print help for the subcommand(s)
```

### 工作目录

程序在运行时需要记录一些配置和文件，因此设置了工作目录，该目录下保存了设备的配置以及声笔输入法的备份，

- MacOS 下工作目录位于 ~/.sbsrf-udpate
- Windows 下工作目录位于 %USERPROFILE%\.sbsrf-update

该目录的结构是这样的：

- .sbsrf-update
  - _cache: 缓存目录，放置从 gitee 或 github 下载的压缩文件，以及解压的文件
  - macos: MacOS 下的默认配置，执行 `sbsrf-update` 时默认读取该配置，没有时会自动创建。该文件根据情况会指向 Squirrel 或 Fcitx5 中的一个
    - config.toml: 配置文件，具体内容见下方
    - backups: 备份目录
      - yyyyMMdd: 版本备份
  - windows: Windows 下默认配置，执行 `sbsrf-update.exe` 时默认读取该配置，没有时会自动创建
    - config.toml
    - backups
      - yyyyMMdd
  - foobar: 通过 `sbsrf-update device add foobar` 添加的远程设备，目前只支持 iOS 上的仓输入法
    - config.toml
    - backups
      - yyyyMMdd
  - ...: 其它仓输入法的设备

### 配置文件

- MacOS 下配置文件位于 ~/.sbsrf-update/macos 下，Windows

```text
# 输入法程序名称
name = "Squirrel"

# 输入法可执行文件路径
exe = "/Library/Input Methods/Squirrel.app/Contents/MacOS/Squirrel"

# 输入法用户目录
user_dir = "/Users/hotleave/Library/Rime"

# 更新信息及备份所在目录
update_dir = "/Users/hotleave/.sbsrf-update/Squirrel"

# 最大备份数量，设置为 0 时不备份
max_backups = 1

# 是否使用整句输入方案
sentence = false

# 当前声笔输入法版本
version = "20240412"
```

## 版本信息

### 0.3.4

- [新增] 支持小企鹅输入法自动部署
- [修复] 解决同一版本更新时重复下载的问题


### 0.3.3

- [调整] 去掉 `-g` 或 `--github` 参数，默认从 github 上下载
- [修复] 有新的发布版本时清理本地缓存，防止要下载的文件同名而不重新下载导致更新错误

### 0.3.2

- [调整] 去掉 update 命令的 `-f` 参数，当本地版本与发布版本一致时询问是否更新

### 0.3.1

- [新增] 支持从 github 更新, 使用 `-g` 或 `--github` 参数，默认使用 gitee

### 0.3.0

- [新增] 支持 MAC 端小企鹅输入法
- [新增] 缓存清理
- [新增] 设备管理命令
- [新增] 检测是否安装输入法程序，如果未安装自动安装
- [调整] 命令行参数格式调整
- [调整] Windows 下工作目录由 %APPDATA%\.sbsrf-update 调整为 %USERPROFILE%\.sbsrf-update
- [调整] 配置文件格式调整，与之前不兼容

### 0.2.1

- [新增] 更新后自动部署
- [修复] Windows下备份时由于小狼毫算法服务正在运行导致无法复制文件而报错的问题

### 0.2.0

- [新增] 支持 iOS 平台设备上的仓输入法更新
- [新增] 支持还原到某个备份版本

### 0.1.0

- [新增] 支持 MacOS 及 Windows 平台声笔输入法的更新
- [新增] 支持更新前备份
