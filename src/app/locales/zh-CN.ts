// 统一维护简体中文文案，避免组件内硬编码文本。
export const zhCN = {
  title: "tt-audio-lab 音频桌面组件",
  subtitle: "M4 性能分层与自动降级阶段（Tauri + Vue + Rust）",
  statusLabel: "运行状态",
  status: {
    idle: "空闲",
    connecting: "连接中",
    running: "运行中",
    paused: "已暂停",
    fallback: "本地配置模式"
  },
  actions: {
    apply: "保存并应用",
    reset: "恢复默认",
    cancel: "取消"
  },
  settings: {
    title: "参数设置",
    open: "设置",
    smoothing: "平滑系数",
    smoothingHelp: "越大越平稳，越小越灵敏",
    gain: "增益",
    gainHelp: "提高整体响应幅度",
    quality: "画质档位",
    qualityHelp: "高负载时会按 Ultra -> High -> Balanced 自动降级",
    qualityUltra: "Ultra（240Hz 优先）",
    qualityHigh: "High（120Hz 稳定）",
    qualityBalanced: "Balanced（60Hz 节能）",
    windowMode: "窗口模式",
    windowModeHelp: "普通模式保留标题栏，组件/覆盖层模式会隐藏标题栏",
    monitor: "目标显示器",
    monitorHelp: "选择窗口默认所在屏幕，留空表示当前屏幕",
    monitorAuto: "当前显示器",
    clickThrough: "点击穿透",
    clickThroughHelp: "仅在桌面组件/覆盖层模式生效，普通模式会自动禁用",
    launchAtStartup: "开机启动",
    launchAtStartupHelp: "当前为预留项，后续版本接入"
  },
  windowMode: {
    normal: "普通窗口",
    desktopWidget: "桌面组件",
    overlay: "悬浮覆盖层"
  },
  metrics: {
    device: "采集设备",
    quality: "当前画质",
    renderer: "渲染后端",
    fps: "FPS",
    frameTime: "当前帧耗时",
    p95: "帧耗时 P95",
    autoDowngrade: "性能压力持续偏高，已自动降级",
    autoDowngradeSaveFailed: "后端保存失败"
  },
  audio: {
    discovered: "已发现设备数"
  },
  visualizer: {
    section: "实时频谱",
    styles: {
      bars: "柱状",
      wave: "波形",
      radial: "环形",
      mirror: "镜像脉冲",
      spiral: "螺旋星轨",
      matrix: "矩阵能量",
      particles: "粒子爆发",
      waterfall: "光谱瀑布",
      radar: "雷达扇扫"
    }
  }
} as const;
