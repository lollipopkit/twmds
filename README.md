## twmds
批量下载 Twitter 媒体文件

### 用法
首先进入想要保存媒体文件的文件夹，在该文件新建文件夹，文件夹的名称是你想要下载的 Twitter 用户的 ID，类似如下：
```shell
.
├── 用户ID1
├── 用户ID2
└── 用户ID3
```

然后在该文件夹下运行 `twmds` 命令，如下：
```shell
twmds
# 可以选择不登录，登录需要先执行 `twmd -L`
twmds -n
```
