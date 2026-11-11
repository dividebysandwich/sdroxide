# sdroxide — 界面的简体中文文本，每个译本都以英文为准进行核对。规则见 crates/sdroxide-ui/src/i18n.rs。
#
# 键名按显示位置分组并据此命名：
# settings-ui-* 指设置窗口的 UI 选项卡，依此类推。凡以大写字母显示的文字，
# 这里也写成大写；程序不会对译文做大小写转换。
#
# 翻译规则与术语约定见 i18n/README.md。

## 语言的自称，显示在语言选择列表中。
language-name = 简体中文

## 来自 sdroxide-types 的取值，按界面上的显示名称翻译（src/i18n/labels.rs）。

layout-mode-auto = 自动
layout-mode-desktop = 桌面
layout-mode-tablet = 平板
layout-mode-small = 小屏幕
layout-mode-phone = 手机

ui-theme-default = 默认
ui-theme-light = 浅色
ui-theme-high-contrast = 高对比度
ui-theme-green-phosphor = 绿色荧光
ui-theme-amber-phosphor = 琥珀色荧光
ui-theme-teal-orange = 青色 / 橙色
ui-theme-rainbow = 彩虹
ui-theme-nord = Nord
ui-theme-nord-dark = Nord 深色
ui-theme-gruvbox = Gruvbox
ui-theme-everforest = Everforest
ui-theme-solarized-dark = Solarized 深色
ui-theme-solarized = Solarized
ui-theme-dracula = Dracula
ui-theme-catppuccin-mocha = Catppuccin 摩卡
ui-theme-catppuccin-latte = Catppuccin 拿铁
ui-theme-modern-minimalist = 现代极简

chrome-style-angled = 斜角
chrome-style-rectangular = 直角
chrome-style-rounded = 圆角
chrome-style-gradient = 渐变
chrome-style-bevel = 3D 浮雕
chrome-style-terminal = 终端

font-size-small = 小
font-size-medium = 中
font-size-large = 大

bandplan-kind-ham = 业余
bandplan-kind-cw = CW
bandplan-kind-digi = 数字
bandplan-kind-phone = 话音
bandplan-kind-beacon = 信标
bandplan-kind-broadcast = 广播
bandplan-kind-am = AM / 长波 / 中波
bandplan-kind-cb = CB

speech-verbosity-terse = 简洁
speech-verbosity-normal = 正常
speech-verbosity-full = 详细

# 括号中的例子是（英文）语音播报的内容。
speech-freq-style-dial = 读表盘（14 点 074）
speech-freq-style-digits = 逐位读出
speech-freq-style-full = 表盘读法带单位

speech-callsign-style-phonetic = 音标（kilo one alpha）
speech-callsign-style-letters = 字母（kay one ay）
speech-callsign-style-as-is = 按原样

## 设置窗口，UI 选项卡（src/app/settings/ui_tab.rs）。

settings-ui-display = 显示
settings-ui-moved-note = 全景图的细节、频谱的反应速度和瀑布图的滚动速度，在显示条上的 SPEC 按钮中设置；窄窗口下在 DISP 菜单中设置。
settings-ui-language = 语言
    .hint = 界面的语言。无论选择哪种语言，F1 手册和语音播报都保持英文。
settings-ui-language-system = 跟随系统（{ $language }）
settings-ui-layout = 布局
settings-ui-theme = 主题
settings-ui-button-style = 按钮样式
settings-ui-window-style = 窗口样式
settings-ui-frame-rate = 屏幕刷新率
settings-ui-palette = 瀑布图调色板
settings-ui-tuning-buttons = 调谐按钮
settings-ui-tuning-buttons-show = 在手机和平板上显示
    .hint = 触摸屏上控制条下方的一排按钮：降低步进 / 步进 / 增加步进。没有可滚动数字的滚轮，也没有旋钮，因此如果没有这些按钮，要移动一个已知步进，只能把整个频率输入进去。在桌面上从不显示。
settings-ui-tuning-step = 步进：{ $step }
settings-ui-tuning-snap = 首次按下时对齐到步进
    .hint = 表盘停在两个步进倍数之间时，按下的方向决定跳到哪一个。例如以 1 kHz 步进时，7 074 300 向上会到 7 075 000，向下会到 7 074 000。之后按钮照常按步进移动。关闭时，每次按下都只移动一个步进。
settings-ui-smoothing = 瀑布图平滑
settings-ui-smoothing-interpolate = 插值
    .hint = 当显示宽度大于变换的分辨率时，把每个屏幕像素与其周围的频点和行混合，使信号看起来是连续的。若要用矩形瀑布图（每个频点一个方块、每行一个方块）从图像上读取信号形状，则应关闭它：插值后的信号与真正更宽的信号无法区分。更大的 FFT（FFT 芯片）是另一半的关键。
settings-ui-clock = 时钟
settings-ui-clock-show = 全景图上显示 UTC 与本地时间
    .hint = 在全景图右上角显示挂钟，同时给出 UTC 和你自己的时间。
settings-ui-background = 频谱背景
settings-ui-background-gradient = 渐变
settings-ui-background-top = 顶部
settings-ui-background-bottom = 底部
settings-ui-spot-colours = 信号标注颜色
    .hint = 每个信号来源的着色：瀑布图底部的方框、SPOTS 列表中的标签，以及世界地图上的圆点。
settings-ui-spot-colour = { $kind } 信号的颜色
settings-ui-spot-colours-reset = 把所有信号颜色恢复为默认值
settings-ui-reset = 重置
settings-ui-bandplan-colours = 频段计划颜色
    .hint = 瀑布图底部频段计划条上，每类频率分配所使用的底色。这些色块以半透明方式叠加在瀑布图上，因此在瀑布图中看起来比这里的色样更深。
settings-ui-bandplan-colour = { $kind } 分配的颜色
settings-ui-bandplan-colours-reset = 把所有频段计划颜色恢复为默认值
settings-ui-skimmer-font = Skimmer 字号
settings-ui-waterfall-font = 瀑布图 / 频谱字号
settings-ui-interface-font = 界面字号
    .hint = 缩放整个界面：菜单、对话框、窗口、电台选项卡、顶部栏及其按钮。上面两个字号都以此为基准。
settings-ui-cities = 地图上的城市
    .hint =
        在平面地图（FT8/WSPR、APRS、ADS-B 和 AIS）上绘制世界城市：每个地点一个圆点，有空间时在旁边标注名称。

        它们是整张世界地图上标示*位置*的主要依据，同时也是地图上最繁忙的内容。如果名称妨碍了你阅读电台，请关闭。3D 地球不受影响：那里的城市是夜间灯光而非标记，也不会有文字压在通联之上。
settings-ui-cities-show = 显示城市及其名称
settings-ui-update-check = 启动时检查更新
    .hint = 每次启动时向 sdroxide.com 询问一次是否发布了更新版本，并在瀑布图上方以可关闭的横幅提示。除请求本身外，不发送任何其他内容。
settings-ui-band-conditions-download = 下载波段传播预报
    .hint = 每小时最多一次从 HAMQSL.com 获取短波波段传播预报，用于为波段菜单着色。关闭后，除非打开 3D 视图，波段菜单不显示预报。除请求本身外，不发送任何其他内容。
settings-ui-broadcast-schedule-download = 下载广播时间表
    .hint = 在新的广播季开始时，从 eibispace.de 下载 EiBi 短波广播时间表（约 1 MB）。关闭后，继续使用 sdroxide 内置的（或上次下载的）时间表；设置 → 信号标注 →“立即下载时刻表”仍可按需下载。
settings-ui-3d-view = 3D 视图
settings-ui-clouds = 云层渲染
settings-ui-clouds-layered = 分层
settings-ui-clouds-volumetric = 体积
settings-ui-clouds-note = 3D 视图中的云层图层如何绘制天气。分层会在对流层中叠加若干切片，开销较小。体积则让光线穿过云层，因此太阳会把云顶的投影投到下方的云层上，雷暴内部的闪电会从中透出光芒，而不仅仅是照亮外壳，代价是每个像素的开销高出数倍。

## 设置窗口，UI 选项卡：语音播报。

speech-title = 语音播报
speech-enabled = 播报电台的变化
    .hint = 朗读发生的变化，以便不看电台也能操作
speech-voice = 语音
speech-voice-shipped = 内置语音
speech-speed = 语速
    .hint = 语音会拉伸或压缩自己的节奏，因此音调不会改变。超过约 2 倍后，语速不再加快。
speech-volume = 音量
speech-output = 输出
speech-detail = 详细程度
speech-duck = 压低接收音量
speech-duck-while = 播报期间
speech-test = 测试
speech-what = 播报哪些内容
speech-cat-frequency = 频率
speech-cat-mode-band = 模式和频段
speech-cat-vfo-split = VFO 和异频
speech-cat-agc-gain = AGC 和增益
speech-cat-levels = 驱动、调谐和话筒
speech-cat-ptt = 发射和接收
speech-cat-rit-xit = RIT 和 XIT
speech-cat-memory-scan = 记忆信道和扫描
speech-cat-band-edge = 离开业余频段
speech-cat-notices = 警告和消息
speech-cat-filters = 滤波器、静噪和降噪
    .hint = 默认关闭：追踪信号时它们会频繁变化
speech-decoded = 解码的消息
speech-ft8-to-me = 呼叫我的 FT8 消息
speech-ft8-cq = 我能回应的 FT8 CQ
    .hint = 20 米波段的繁忙时段，每分钟可能有上百条
speech-js8 = 发给我的 JS8 消息
speech-js8-allcall = 也包括 JS8 @ALLCALL
speech-fsq = 发给我的 FSQ 消息
speech-include-snr = 包括信号报告
    .hint = 仅当消息本身不含报告时才加入——已经报告了数值的解码结果，不再额外加上我们的报告
speech-ft8-qso = 我自己的 FT8 通联
    .hint = 序列器即将发送的内容，以及通联结束
speech-reading-text = 朗读解码的文字
speech-text-cw = CW
speech-text-rtty-psk = RTTY、PSK、Olivia、THOR、FSQ
speech-text-cw-locked = 仅当解码器锁定时朗读 CW
    .hint = 朗读未锁定解码器的输出，还不如保持安静
speech-text-note = 两者默认都关闭。解码器生成文字的速度快于语音朗读，因此任何落后于实时音频太多的内容都会被丢弃，而不是排队等待。
speech-tuning-up = 调谐时
speech-swr-while-tuning = 按住 TUNE 时读出驻波比
speech-swr-every = 每隔
speech-swr-summary = 松开时报告最佳匹配
speech-swr-alarm = 任何发射期间出现高驻波比时发出警告
speech-how-read = 读法
speech-frequencies = 频率
speech-callsigns = 呼号
speech-quiet-on-tx = 发射时保持安静
    .hint = 语音从你的扬声器播出，因此会被你的话筒拾取。高驻波比警告仍会播报。
speech-device-note = 播报使用其独立的声音设备，因此永远不会被录制，也不会发送给远程收听者。播报状态、重复上一条播报以及句中停止的按键，设在“控制”选项卡中。

## 设置窗口（src/app/settings/mod.rs）

settings-waiting-station-s-network-configuration = 等待电台的网络配置……
settings-device-reports = 按设备报告的值
settings-transverters = 变频器
settings-one-row-per-box-while = 每个方框一行。当表盘位于某行的频段内时，电台会调谐到“表盘 + 偏移”，并应用该行的发射规则和驱动上限；在所有行之外，电台使用其自身的频段。各行按从上到下的顺序判断。
settings-name = 名称
settings-band-low = 频段下限
settings-band-high = 频段上限
settings-offset = 偏移
settings-transmit = 发射
settings-max-drive = 最大驱动
settings-off-takes-transverter-out-line = 关闭后，该变频器退出线路但保留此行——频段恢复为电台自身能覆盖的范围。
settings-t-2-m = 2 米
settings-what-you-call-shown-log = 你给它起的名字。表盘选中它时显示在日志中。
settings-bottom-band-transverter-works-dial = 此变频器工作的频段下限，以表盘读数表示。2 米为 144。
settings-top-band-dial-148-2 = 频段上限，以表盘读数表示。2 米在第 2 和第 3 区为 148，第 1 区为 146。
settings-radio-ends-up-dial-offset = 电台最终调谐到“表盘 + 偏移”，因此把频段下变到中频的变频器为负值：2 米变到 28 MHz 中频时为 -116，因为 144 - 116 = 28。对于略有偏差的本振，可拖动以每次微调一赫兹。
settings-off-while-converting-receive-only =
    转换时关闭：仅接收——此行是变频器而非变频收发器，该频段上不会有任何发射。

    通过同一变频器时：双向工作，发射使用相同的偏移。
settings-ceiling-transmit-drive-while-transverter =
    选中此变频器时发射驱动的上限，以满量程的百分比表示。变频器的中频输入只能承受毫瓦级功率，而适用于电台自身频段的驱动会将其烧毁，因此这是本行最重要的字段。100% 表示不设限制。

    你在驱动设置中的数值会被压到此上限之下，而不是被改写，因此离开该频段时，你在 HF 上使用的数值会恢复。
settings-remove = 删除
settings-add-transverter = 添加变频器
settings-takes-effect-apply-reconnect-like = 在“应用”或重新连接后生效，与上面的变频器偏移相同。
settings-maximum-transmit-drive = 最大发射驱动
settings-hard-limit-drive-tune-controls = 驱动和 TUNE 控件无法超越的硬性上限，在下方的频段校准之后应用，因此任何设置都不能把驱动推回其上。请将其设在电台的额定功率处，这样整个驱动控件都能有效使用——对于自行调制采样的发射机（HPSDR、LimeSDR、PlutoSDR、HackRF），满驱动就是发射机全开，对于高增益功放而言，远超其末级的额定值。关闭则控件可达满驱动。
settings-limit-drive = 驱动限制为
settings-per-cent-full-drive-key = 满驱动的百分比。将电台接入假负载并按下发射，然后把此值调低，直到功率计读数达到功放的额定功率。
settings-applies-immediately-every-band-tune = 立即生效，适用于每个频段，也包括 TUNE。
settings-transmit-drive-band = 分频段的发射驱动
settings-every-amplifier-has-different-gain = 每台功放在每个频段上的增益都不同，因此同一个驱动设置在各频段产生的功率也不同。请在每个频段上测量输出，然后把输出偏高的频段调低，直到全部一致：以需要最多的频段（通常是最高频段）设定驱动，其余频段则降到与之相同的水平。单位为输出功率的分贝，同时适用于话音、数字和 TUNE。全部为零（默认值）则不做任何改变。
settings-other = 其他
settings-clear-all = 全部清除
settings-back-no-calibration-drive-setting = 恢复为无校准：驱动设置完整地作用于每个频段。
settings-applies-immediately-band-you-would = 立即生效，并作用于你将要发射的频段——通过变频器时，即表盘所在的频段，而变频器自身的驱动上限仍优先于此处的任何设置。
settings-settings = 设置
settings-enter-server-s-address-first = 请先输入服务器地址
settings-enter-receiver-s-address-first = 请先输入接收机地址，或从公共 SDR 中选择一个
settings-no-radio-selected-press-discover = 未选择电台——请按“搜索”，或输入地址
settings-enter-radio-s-address-first = 请先输入电台地址
settings-general = 常规
settings-radio = 电台
settings-ui = 界面
settings-alerts = 提醒
settings-controls = 控制
settings-spots = 信号标注
settings-freedv = FreeDV
settings-uploads = 上传
settings-winlink = Winlink
settings-servers = 服务器
settings-t-r-switch = 收发开关
settings-amplifier = 功放
settings-remote = 远程
settings-tle = TLE
settings-profiles = 配置文件
settings-station = 电台站点
settings-enter-digital-mode-ft8-sstv = 输入一次数字模式（FT8 / SSTV / …）即可加载已保存的值。
settings-callsign = 呼号
settings-grid-square = 网格定位
settings-your-callsign-grid-shared-across = 你的呼号和网格定位，在 FT8/FT4/FT2、SSTV 图像头以及日志中共用。也可在 FT8 / SSTV 设置对话框中编辑。
settings-iaru-region = IARU 区域
settings-which-radio-interface-server-uses = 服务器使用哪种电台接口，在其所在的机器上设置。
settings-radio-interface = 电台接口
settings-waiting-machine-radio-attached-interface = 等待电台所连接的机器。其接口设置在下方，在此期间可以在此处修改。
settings-converter = 变频器
settings-manual-type-offset-below = 手动——在下方输入偏移
settings-what-box-reads-when-offset = 当偏移不是预设值之一时，此框显示的内容。请在下方的“偏移”字段中输入来设置；此处无法选择。
settings-frequency-converter-front-receiver-pick = 接收机前端的频率变换器。请选择一个，或在旁边输入偏移。
settings-how-far-converter-moves-signal =
    变频器把信号移向接收机的距离，单位为赫兹——与所有变频器的说明文档和其他 SDR 程序所用的数值及符号相同。电台最终调谐到“表盘 + 偏移”：上变频器为正（Ham It Up 为 125000000），把频段下变到中频的设备为负，例如卫星 LNB，或 28 MHz 中频上的 2 米变频器，即 -116000000（28 − 144）。0 表示无变频器。

    以赫兹为单位逐次微调，这是本振略有偏差的变频器所需要的。

    这是接收通路。发射线路中的设置见下一行。

    在“应用”后生效。
settings-what-transmit-line-while-converter =
    设置了变频器时，发射线路中的内容。

    转换时关闭：不发射任何内容——这是安全的默认值，适合仅接收的附件。

    通过同一变频器时：一个框双向转换，即变频器。发射使用上面的偏移，并随其微调而变化。

    独立偏移：发射线路与接收线路不同。零是常见情况——接收使用变频器（LNB、Ham It Up），而发射机使用自己的天线，这就是 QO-100 电台的配置。

    业余频段检查仍然适用，以你发射所用的频率为准。

    在“应用”后生效。
settings-how-far-transmit-line-moves =
    发射线路移动信号的距离，单位为赫兹，符号规则与接收偏移相同：电台以“表盘 + 偏移”发射。

    0 = 发射机接入自己的天线，并在表盘所示频率上工作——即 QO-100 电台的 2.4 GHz 上行。

    如果发射变频器把中频*向上*转换，则为负值：电台在表盘以下工作。
settings-rx-range = 接收范围
settings-which-frequencies-radio-receives-mhz =
    此电台可接收的频率，单位 MHz：144-146、430-440。留空则使用设备自行报告的范围。

    超出范围的频段按钮呈灰色，表盘也无法调到该处。

    这些是表盘频率。设置了变频器时，请填写你所调谐的频段——2 米变频器填 144-148，而不是电台实际所在的中频。

    在“应用”后生效。
settings-tx-range = 发射范围
settings-which-frequencies-radio-transmits-mhz =
    此电台可以发射的频率，单位 MHz：144-146、430-440。留空则使用设备报告的范围——如果设备没有报告，则以驱动程序所言为准，允许任何频率。

    与上面的接收范围一样，为表盘频率。

    这是你自行设定的限制，而不是许可：除非你在 config.toml 中关闭了该项，否则在业余频段之外发射一律会被拒绝。对于仅接收的设备，此设置也不会为其增加发射机。

    在“应用”后生效。
settings-antenna = 天线位于
settings-where-radio-listens-station-antenna =
    此电台从何处收听。

    在电台所在地：天线是你自己的，因此你自己的网格定位就说明了它的位置。这是默认值，适用于机房内的一切。

    在别处：在线接收机，或你架设在山顶的设备。它听到的内容会通过旁边的网格定位报告给 PSK Reporter、WSPRnet 和 FreeDV Reporter，而不会用你自己的定位；ADS-B 也以此确定飞机的位置。

    在“公共 SDR”中选择接收机会自动填入此项。

    在“应用”后生效。
settings-receiver-s-maidenhead-locator-jn88ec =
    接收机的梅登海德网格定位——例如 JN88ec、DO30db。四个或六个字符。

    如果你不知道接收机的位置，请留空：此时不会报告任何内容，这是唯一诚实的做法。若用你自己所在的方格报告别人的收听结果，会把它标在错误的大洲上。
settings-audio = 音频输出到
settings-gain-per-band = 分频段增益
settings-remember-receive-gain-per-band = 记住各频段的接收增益
settings-each-band-keeps-receive-gain = 每个频段都保留你上次为它设置的接收增益，表盘回到该频段时会恢复——无论是点击频段按钮、拖过频段边缘，还是转动电台自身的旋钮。全频段覆盖分为 HF、VHF、UHF 及以上。未设置增益的频段沿用当前生效的值。关闭时，所有频段使用同一个增益。
settings-s-meter-calibration = S 表校准
settings-added-radio-s-s-meter =
    加到此电台的 S 表读数上。SDR 的表头起初显示的是 dBFS——即相对于变换器满量程的电平——因此增益很大的接收机在没有接天线时也可能显示 S9。请把此值设为表头所示读数与你已知电平之间的差值：例如信号发生器，或你确信稳定在 S2–S3 左右的安静频段。立即生效。

    叠加在 config.toml 中的 cal_offset_db 之上，后者是整个电台的一个统一数值。
settings-t-0-db = 0 dB
settings-antenna-somewhere-else-no-locator = 天线位于别处且未设置网格定位，因此此电台收听到的内容不会报告给 PSK Reporter、WSPRnet 或 FreeDV Reporter。请在上面输入接收机的网格定位，即可以它的位置报告。
settings-ranges-are-mhz-low-high = 范围以 MHz 为单位，写作“低-高”，用逗号分隔：144-146、430-440——即 144000000-146000000 Hz 和 430000000-440000000 Hz。它们是表盘频率：变频器偏移不会改变它们。上面的变频器偏移是以赫兹为单位的字段。留空某个范围则使用设备自行报告的范围；设备若不报告任何内容，则以其所言为准。
settings-careful-rtl-sdr-blog-v4 = 使用 RTL-SDR 时要小心：Blog V4 自己会在 28.8 MHz 以下做上变频，因此若负偏移把硬件调到那里，频率会被移动两次。
settings-transmit-audio = 发射音频
settings-use = 使用中
settings-every-stage-off = 所有级均关闭
settings-open-tx-audio = 打开发射音频……
settings-microphone-processing-chain-gate-eq = 话筒处理链——噪声门、均衡器、去齿音器、压缩器和限幅器——并附带测试录音，无需发射即可凭听感进行调节。仅适用于话音模式。也可以点击发射框中的“话筒”打开。
settings-pick-radio-interface-above-configuration = 请在上方选择一个电台接口（此配置使用的是已移除的自动检测模式）。
settings-configuration-used-removed-auto-detect = 此配置使用的是已移除的自动检测模式。待电台所连接的机器作出应答后，请在上方选择一个接口。
settings-radio-has-no-interface-yet = 此电台尚无接口——请在上方选择一个，然后按“应用 / 重新连接”。
settings-radio-has-no-interface-yet-2 = 此电台尚无接口。待电台所连接的机器作出应答后，可在上方选择一个。
settings-apply-reconnect = 应用 / 重新连接
settings-switch-interface-now-no-restart = 立即切换到此接口——无需重启
settings-switch-server-s-radio-interface = 立即将服务器的电台切换到此接口——无需重启。若无法打开新接口，则旧接口继续运行。
settings-switches-live-radio-without-restarting = 在不重启的情况下切换正在运行的电台。
settings-everything-above-server-s-own = 以上所有内容都是服务器自身的配置，更改会保存在那里。大多数设置在你更改时即时生效；而在打开设备时才确定的项目——采样率、地址、接口本身——需要使用此按钮。
settings-dx-cluster-telnet = DX 集群（telnet）
settings-enabled = 启用
settings-host = 主机
settings-port = 端口
settings-login-call = 登录呼号
settings-commands = 命令
settings-one-per-line-e-g = 每行一条，例如 SET/FT8
settings-reverse-beacon-network = 反向信标网络（RBN）
settings-read-world-s-cw-rtty = 读取全球 CW/RTTY 的 skimmer 数据，并用它们填充传播地图。正是这一点让地图能够显示此电台并未在收听的频段。默认开启：它不向空中发射任何内容，无需账户，并使用“常规”选项卡中的呼号。RBN 信号不会出现在信号列表中——它们是测量数据，而不是邀请。
settings-t-7000-cw-rtty-feed-7001 = 7000 为 CW/RTTY 数据源，7001 为 FT8/FT4 数据源
settings-one-per-line-e-g-2 = 每行一条，例如 set/filter cont=eu
settings-sent-after-login-place-narrow = 登录后发送。用于缩小数据范围的地方——若不加过滤器，这就是地球上的每一台 skimmer。
settings-rbn-paths-are-placed-country = RBN 路径是按国家中心而非网格定位标注的——对小国较准确，对大国则偏差很大。它们是传播地图上的一个独立图层，可以在那里关闭。
settings-pota-sota-psk-reporter = POTA / SOTA / PSK Reporter
settings-pota-activator-spots = POTA 活动台信号
settings-sota-spots = SOTA 信号
settings-psk-reporter-current-band = PSK Reporter（当前频段）
settings-upload-my-ft8-ft4-ft2 = 上传我的 FT8/FT4/FT2/JS8 解码结果
settings-report-what-station-hears-pskreporter = 将此电台听到的内容报告到 pskreporter.info，使其作为接收机显示在那里。使用“常规”选项卡中的呼号和网格定位。
settings-antenna-2 = 天线
settings-collector = 收集器
settings-t-4739-live-collector-14739-test = 4739 为正式收集器，14739 为测试收集器
settings-max-age-s = 最长保留时间（秒）
settings-wsprnet = WSPRnet
settings-upload-my-wspr-decodes = 上传我的 WSPR 解码结果
settings-send-every-wspr-reception-wsprnet = 将每一次 WSPR 接收结果发送到 wsprnet.org。默认开启：它不向空中发射任何内容，而报告你所听到的内容，正是使 WSPR 接收机成为网络的一部分、而非私人玩具的原因。没有解码到任何内容的时段也会被报告，网络借此区分频段关闭与接收机已关机。
settings-download-who-heard-me = 下载收听过我的电台
settings-ask-wsprnet-org-which-stations = 询问 wsprnet.org 哪些电台解码了此电台的信号。WSPR 没有任何形式的确认机制，因此这是发射信标了解自身覆盖范围的唯一途径。
settings-ask-every = 每隔
settings-looking-back = 回溯查询
settings-apply = {" "}应用{" "}
settings-persist-re-connect-feeds = 保存并（重新）连接数据源
settings-broadcast-stations = 广播电台
settings-winlink-account = Winlink 账户
settings-password = 密码
settings-winlink-account-password-not-gateway = 是 Winlink 账户的密码，而不是网关的密码。区分大小写——请与签发时完全一致地输入。
settings-locator = 网格定位
settings-how-connect = 连接方式
settings-route = 路由
settings-internet = 互联网
settings-forward-cms-over-internet = 通过互联网经 CMS 转发
settings-radio-packet = 电台（分组）
settings-call-rms-gateway-air-radio = 通过无线电呼叫 RMS 网关。电台必须处于 PACKET 或 PACKET-HF 模式。
settings-gateway = 网关
settings-via = 经由
settings-digipeaters-order-separated-spaces-usually = 数字中继，按顺序排列，以空格分隔。通常留空——你能直接收听到的网关，就应该直接呼叫。
settings-speed = 速率
settings-most-rms-gateways-answer-1200 = 大多数 RMS 网关以 1200 应答。以 9600 呼叫 1200 网关，听起来与网关已离线完全相同，因此请以网关所有者公布的数值为准，而不要猜测。9600 还要求两端的电台都使用数据端口。300 适用于 HF。
settings-frequency = 频率
settings-clear = 清除
settings-zero-leaves-dial-alone-which = 填零则不改动表盘，这正是你停留在某一个信道上时所需要的。其他任何值都会在会话开始时调谐电台。
settings-my-gateways = 我的网关
settings-winlink-s-published-gateway-list = Winlink 公布的网关列表需要 sdroxide 没有的 API 密钥，因此请保留你自己的列表。从某一地点能够连到的两三个网关是通过尝试得知的，它们很少变动。
settings-use-2 = 使用
settings-call-one-next-connect = 下次连接时呼叫此网关
settings-current-dial = 当前表盘
settings-forget = 删除
settings-forget-gateway = 删除此网关
settings-add-gateway = + 添加网关
settings-save-gateway-above-list = 将上面的网关保存到列表中
settings-internet-gateway = 互联网网关
settings-cms-address = CMS 地址
settings-client-name = 客户端名称
settings-winlink-s-production-servers-only = Winlink 的正式服务器只接受其认可的客户端名称，对未知名称会回复“正式服务器不允许未知客户端类型”。在 sdroxide 向 Winlink 开发团队注册之前，连接需要使用他们认可的名称。
settings-automatic-connection = 自动连接
settings-connect-timer = 定时连接
settings-every = 每隔
settings-persist-winlink-account = 保存 Winlink 账户
settings-callsign-lookup = 呼号查询
settings-provider = 服务商
settings-auto-fill-name-qth-grid = 点击信号或通联时自动填入姓名 / QTH / 网格
settings-choose-provider-enter-login = 请选择一个服务商以输入其登录信息。
settings-qrz-user = QRZ 用户名
settings-qrz-pass = QRZ 密码
settings-hamqth-user = HamQTH 用户名
settings-hamqth-pass = HamQTH 密码
settings-one-hamqth-account-does-both = 一个 HamQTH 账户可兼作两用——此处的登录信息即 HamQTH 上传选项卡所使用的那一个。
settings-upload = 上传
settings-auto-upload-each-new-qso = 自动上传每一次新通联
settings-master-switch-which-services-qso = 总开关。通联上传到哪些服务，在下面各个选项卡中设置。
settings-turn-auto-upload-each-new = 请先打开上面的“自动上传每一次新通联”——在此勾选的服务，在此之前不会被推送。
settings-auto-upload-off-above-so = 上面的自动上传已关闭，因此各服务的勾选已被禁用，目前不会自动推送任何内容——日志中每次通联的“上传”按钮仍然可用。
settings-qrz-log-key = QRZ 日志密钥
settings-qrz-logbook-api-key-your = 来自你 QRZ 日志设置的 QRZ 日志 API 密钥——不是 XML 查询的登录信息。
settings-eqsl-user = eQSL 用户名
settings-eqsl-pass = eQSL 密码
settings-eqsl-qth = eQSL QTH
settings-qth-nickname-eqsl-profile-upload = 用于上传和下载的 eQSL 资料的 QTH 昵称——仅当账户拥有多个资料时才需要；否则请留空。
settings-same-login-hamqth-callsign-lookup = 与 HamQTH 呼号查询相同的登录信息——填写其中任何一个，两者都会填上。
settings-club-log-email = Club Log 邮箱
settings-club-log-pass = Club Log 密码
settings-club-log-key = Club Log 密钥
settings-wrl-api-key = WRL API 密钥
settings-developer-api-key-world-radio = 来自 World Radio League → Integrations → Developer API 的开发者 API 密钥。它在生成时只显示一次，请当场复制。通联会进入你的默认日志——如果你保留了多个日志，请在 WRL 中设定一个默认值。
settings-confirmations-download = 确认信息（下载）
settings-lotw-user = LoTW 用户名
settings-lotw-pass = LoTW 密码
settings-lotw-upload-uses-tqsl-export = LoTW 上传使用 TQSL——从日志导出 ADIF 并签名。此处下载 LoTW/eQSL 的确认信息，用于标记已通联与已确认。
settings-sync-confirmations = 同步确认信息
settings-fetching-subscriptions = 正在获取订阅……
settings-spectrum-waterfall-everything-read-off =
    它的频谱、瀑布图以及从中读取的一切，都在该电台的选项卡上，因此它自己没有这些。上面的界面设置仍属于此电台并继续生效——更改后请按“应用 / 重新连接”，两个电台会一起恢复。

    要让它重新独立工作，请清空该电台页面上的全景接收机框。
settings-panadapter = 全景图
settings-give-radio-wideband-panadapter-borrowing = 通过借用另一台电台的接收机，为此电台提供宽带全景图——例如同一天线上的 SDR，或由此电台的中频输出馈送的接收机。频谱、瀑布图、副接收机、数字模式和 skimmer 都来自该接收机；表盘、模式、滤波器和发射机仍保留在此。
settings-receiver = 接收机
settings-none = 无
settings-which-radio-receives-one-leaves =
    为此电台接收的是哪一台电台。出借期间它会离开选项卡栏，因为其前端属于该电台的引擎；将此项设为“无”后，它会立即回到选项卡栏。

    在“应用”后生效。
settings-connected = 连接至
settings-how-far-above-radio-s =
    接收机位于此电台表盘之上的距离，单位为赫兹——同一天线上的接收机为 0。对于中频抽头，则为接收机所停留的中频：K3 的 KXV3B 为 8215000，9 MHz 中频为 9000000。

    这不是上面的“变频器”字段（它会重新调谐电台），也不是“CAT”选项卡中的 I/Q 中心偏移（它针对的是此电台自身的声卡）。此处输入的内容永远不会发送给任何一台电台。

    在“应用”后生效。
settings-audio-2 = 音频来自
settings-follow-dial = 跟随表盘
settings-keep-receiver-radio-s-dial = 让接收机跟随此电台的表盘，并在全景图上调谐时移动表盘。关闭时接收机停在原处——电台去了别处，而接收机固定监视某一段。
settings-invert-spectrum = 反转频谱
settings-mirror-receiver-s-span-about = 以接收机的中心为轴镜像其跨度，适用于本振位于信号上方、把频段倒着送过来的抽头。除非瀑布图上充满了全都位于表盘另一侧的信号，否则请保持关闭。
settings-mute-transmit = 发射时静音
settings-silence-receive-audio-while-radio = 此电台发射时，静音接收音频。默认开启：接收机与发射机共用同一天线，或接在此电台的中频上时，它在通话期间听到的就是你自己的发射机。在电台本身按下的通话——话筒的 PTT、脚踏开关、MOX——同样计入，只要 CAT 连接报告电台已转为发射。
settings-blank-transmit = 发射时清空显示
settings-stop-panadapter-waterfall-while-radio = 此电台发射时，停止全景图和瀑布图。默认开启：发射机会把整个跨度都涂满，从而抹掉其背后的频段。关闭后，你可以在通话期间观察频段——或你自己的信号——这在接收机使用独立天线时很有价值。仅适用于由 sdroxide 发射的通话；在电台上按下发射的通话会被完整接收，因此无论此项如何设置，画面都会持续刷新。
settings-per-mode-offsets-radio-whose = 按模式设置偏移，适用于中频随模式变化的电台。留空的模式使用上面的偏移。
settings-computer = 本计算机上
settings-add-radio-computer = 在本计算机上添加电台
settings-radio-s-settings-are-below = 此电台的设置在下方。拖动可沿条带移动它。
settings-switch-radio-dialog-follows-drag = 切换到此电台（对话框随之切换）。拖动可沿条带移动它。
settings-tx = ● 发射
settings-station-radio = 主台：
settings-station-radio-tip = 本站的主电台。它运行本站的网络服务（Spot 源、旋转器、功放、收发切换器），并接收命令行设置，因此始终留在列表中。要让它休息，请用“连接”关闭；要换用其他硬件，请在下方选择另一个接口。
settings-mute-radio-s-audio = 静音此电台的音频
settings-link = 连接
settings-close-radio-configuration-kept = 关闭此电台（保留其配置）
settings-close-tab-close-radio-station = 关闭此选项卡，或在其所在的电台站点上关闭电台
settings-close-tab = 关闭此选项卡
settings-transverters-full = { $max } 是此表最多可容纳的数量
settings-dialling = 正在拨号 { $url }……
settings-region-note = 电台所在的区域。它决定所有频段计划：频段边界、瀑布图条带上的 CW / 数据 / 话音子频段、skimmer 的收听频率、提供的呼叫频率，以及发射时哪些算作频段外。在第 { $region } 区，这意味着 70 厘米 { $region ->
        [1] 为 430–440 MHz——因此 446 属于频段外，40 米止于 7.200
        [2] 为 420–450 MHz，40 米延伸至 7.300
       *[3] 为 430–450 MHz，80 米止于 3.900
    }。立即生效，并适用于此电台站点上的每一台电台。
settings-rx-grid-reported = 经由此电台的接收结果，以 { $grid } 为位置报告，而不是以你自己的网格定位报告。
settings-rx-grid-invalid = { $grid } 不是梅登海德网格定位，因此此电台收听到的内容不会被报告。应为四个或六个字符：两个字母、两个数字，以及可选的另外两个字母。
settings-bypassed = 已旁路
# $state 是 settings-bypassed、settings-use 或 settings-every-stage-off 之一。
settings-tx-audio-chain = 处理链{ $state }。
settings-gateway-via = { " " }经由 { $via }
settings-auto-upload-to = 将每一次新通联自动上传到 { $service }
settings-tle-updated = 已更新 { $n } 个订阅：共 { $total } 颗卫星。
settings-tle-updated-failed = 已更新 { $ok } / { $n }；{ $failed } 个失败——请见上方各行。
settings-panadapter-of = 此电台是 { $radio } 的全景图
settings-radio-n = 电台 { $n }
settings-on-station = 位于 { $station }
settings-add-radio-on-station = 在 { $station } 上添加电台——电台站点会立即为其提供服务
settings-panadapter-receiver-for = { $radio } 的全景接收机
settings-hang-up-stays-on = 挂断。电台仍保留在 { $station } 上。
settings-close-on-station = 要在 { $station } 上关闭 { $radio } 吗？其配置仍保留在该机器上；此处的选项卡随之关闭，其他人的也一样。
settings-close-it-on = 在 { $station } 上关闭它
settings-locator-hint = 网格定位

## 设置窗口，警报（src/app/settings/alerts.rs）

settings-alerts-audible-alerts = 声音提醒
settings-alerts-sound-alarm-when-decode-matters = 当解码结果重要时发出警报声
settings-alerts-plays-over-own-audio-output = 通过其独立的音频输出播放，因此即使频段所用的扬声器与此屏幕不同，也能听到。
settings-alerts-volume = 音量
settings-alerts-output = 输出
settings-alerts-test = 测试
settings-alerts-what-sound = 发出哪些声音
settings-alerts-each-station-quiet-while-after = 每个电台在一次提醒之后会安静一段时间，因此繁忙的频段不会在每个时隙都响起。“语音”和“音调 + 语音”由语音播报朗读——在“设置 → UI”中开启它。

## 设置窗口，功放（src/app/settings/amp.rs）

settings-amp-amplifier = 功放
settings-amp-waiting-station-s-amplifier-configuration = 正在等待电台站点的功放配置……
settings-amp-hobbypcb-hardrock-500-hardrock-50 = 通过其 USB 串口连接的 HobbyPCB Hardrock-500 或 Hardrock-50。SDRoxide 在 AMP 窗口中读取其仪表、切换其频段、模式和天线，并操作其调谐器。功放仍像以前一样由电台的 PTT 线路控制。
settings-amp-enable = 启用
settings-amp-model = 型号
settings-amp-untested-panel-has-only-been = 未经测试：此面板仅在 Hardrock-500 上测试过。此型号的表头限值由 Hardrock-500 的数值换算而来，可能有误。请留意功放自身的显示。
settings-amp-port = 端口
settings-amp-choose = — 请选择 —
settings-amp-baud = 波特率
settings-amp-must-match-amplifier-s-menu = 必须与功放的菜单设置一致。HobbyPCB 自己的控制面板需要 115200。
settings-amp-follow-radio-s-band = 跟随电台的频段
settings-amp-sends-transmit-frequency-amplifier-every = 每次频段变化时，都会把发射频率发送给功放，使其滤波器跟随电台，而无需单独的 CAT 线缆。发射期间从不发送。
settings-amp-show-power-swr-tx-meter = 在发射表上显示其功率和驻波比
settings-amp-only-where-radio-does-not = 仅在电台自身不测量它们时显示，并且仅在功放接入线路时显示。
settings-amp-off = 关闭
settings-amp-connecting = 正在连接……
settings-amp-status = 状态
settings-amp-apply = {" "}应用{" "}
settings-amp-save-re-open-amplifier-s = 保存并（重新）打开功放的端口

## 设置窗口，控制（src/app/settings/controls.rs）

settings-controls-keyboard = 键盘
settings-controls-click-shortcut-rebind-then-press = 点击某个快捷键即可重新绑定，然后按下组合键（Esc 取消）。在文本框中输入时，快捷键不生效。
settings-controls-shortcut = 快捷键
settings-controls-does = 功能
settings-controls-step-mode = 步进 / 模式
settings-controls-accel = 加速
settings-controls-on = 开
settings-controls-press-key = 按下一个键……
settings-controls-remove-binding = 删除此绑定
settings-controls-add-shortcut = + 添加快捷键
settings-controls-restore-defaults = 恢复默认值
settings-controls-bind-hold-talk-space = 将按住通话绑定到空格键
settings-controls-hold-space-transmit-releasing-losing = 按住空格键发射；松开它——或窗口失去焦点——即停止发射
settings-controls-unkey-held-ptt-after = 在以下时间后自动松开按住的 PTT
settings-controls-backstop-against-stuck-key-controller = 防止按键卡住或控制器停止上报的后备机制。0 表示禁用。
settings-controls-panadapter-mouse = 全景图鼠标
settings-controls-wheel = 滚轮
settings-controls-wheel-shift = 滚轮 + Shift
settings-controls-tune-step = 调谐步进
settings-controls-zoom-rate = 缩放速率
settings-controls-click-tune-rounding = 点击调谐取整
settings-controls-invert-wheel-direction = 反转滚轮方向
settings-controls-left-drag-tunes-well-pans = 左键拖动同时调谐和平移
settings-controls-off-makes-left-drag-pan = 关闭后，左键拖动仅平移视图，与右键拖动相同。
settings-controls-scroll-digit-frequency-readout-tune = 滚动频率读数上的数字即可调谐
settings-controls-both-wheel-actions-are-tune = 两种滚轮操作都是调谐——此时已没有缩放的方式。
settings-controls-restore-mouse-defaults = 恢复鼠标默认值
settings-controls-mouse-buttons = 鼠标按键
settings-controls-left-right-buttons-are-reserved = 左键和右键保留用于调谐和平移；中键和额外按键可自由设置。为 PTT 按住的侧键，其作用与脚踏开关相同。
settings-controls-add-mouse-button = + 添加鼠标按键
settings-controls-f1-always-opens-manual-even = F1 始终打开此手册，即使在输入文字时也是如此，因此无法重新绑定。
settings-controls-midi-controller = MIDI 控制器
settings-controls-midi-controllers-need-native-app = MIDI 控制器需要原生应用——浏览器客户端无法访问 MIDI。
settings-controls-any-class-compliant-midi-surface = 任何符合 class 标准的 MIDI 设备都可以使用：DJ 控制器的转轮可以做很好的 VFO 旋钮，其按键可做 PTT 和频段按钮，其推子可做增益控制。
settings-controls-enable = 启用
settings-controls-controller = 控制器
settings-controls-feedback = 反馈至
settings-controls-rescan-ports = 重新扫描端口
settings-controls-not-connected = 未连接。
settings-controls-move-control-see-here = 移动一个控件即可在此看到它。
settings-controls-control = 控件
settings-controls-reads = 读取为
settings-controls-led = LED
settings-controls-move = 移动它……
settings-controls-click-then-move-control-you = 点击，然后移动你想要绑定的控件
settings-controls-speed-sensitivity-spin-faster-tune = 速度灵敏度：转得越快，调谐越快
settings-controls-send-current-value-back-light = 将当前值回传，用于点亮 LED 或移动推子
settings-controls-add-midi-control = + 添加 MIDI 控件
settings-controls-clear-all = 全部清除
settings-controls-endless-jog-encoders-send-relative = 无限旋转（转轮）编码器发送的是相对步进而非位置，有三种编码方式，在小幅移动时看起来很相似。“学习”会根据顺时针旋转进行推测；如果旋钮随后调谐的方向相反，请勾选“反向”。
settings-controls-none = — 无 —

## 设置窗口，常规（src/app/settings/general.rs）

settings-general-system-default = 系统默认
settings-general-remote-access = 远程访问
settings-general-signed-in-station = 已登录的电台
settings-general-sign-out = {" "}退出登录{" "}
settings-general-sign-out-forgets-device = 清除此设备的登录。下次连接时电台会再次询问；当前连接在你关闭之前保持不变。
settings-general-password-kept-no-cookie = 由于未提供登录 Cookie，此浏览器正在记住 { $stations } 的密码。旧版电台会这样做；这与 Cookie 不同。
settings-general-what-remote-client-browser-page = 远程客户端（浏览器页面，或以 --connect 启动的另一个 sdroxide）在此电台站点允许其操作之前，必须提供的凭据。在服务器模式（--server）下生效；下次登录时即会应用更改，无需重启。
settings-general-username = 用户名
settings-general-password = 密码
settings-general-clients-must-give-password-leaving = 客户端必须提供密码。用户名留空也可以。
settings-general-clients-must-sign = 客户端必须登录。
settings-general-empty-anyone-who-can-reach = ⚠ 留空：任何能访问服务器端口的人都可以以你的呼号操作此电台。转发端口之前，请先设置密码。
settings-general-stored-clear-config-toml-like = 以明文保存在 config.toml 中，与 sdroxide 保存的其他密码相同。
settings-general-receive-audio-gain = 接收音频增益
settings-general-t-0-db = 0 dB
settings-general-extra-gain-everything-radio-sends =
    在音量控制之上，对此电台发送到扬声器的所有内容额外增益。除非电台在满音量下仍然很小，否则请保持 0 dB：音量滑块的顶端是原始到达的音频，因此它只能把电台调小，而不能调大；而且某些收发信机的 USB 声音输出远低于满量程。

    每次增加 6 dB。过大会削波——音频在满量程处被限幅，而不会回绕，因此过度增益听起来是刺耳而非响亮。录音在此之前获取，不受影响。
settings-general-reload-band-plan = 重新加载频段计划
settings-general-re-read-bandplan-json-machine = 在电台所连接的机器上重新读取 bandplan.json 并应用它——频段边界、子频段和 skimmer 窗口——无需重启。
settings-general-running-station-s-own-band = 正在使用电台站点自己的频段计划。
settings-general-running-built-iaru-tables = 正在使用内置的 IARU 表。
settings-general-settings-file = 设置文件
settings-general-settings-are-machine-radio-attached = 设置保存在电台所连接的机器上。请在那里导出。
settings-general-export = 导出……
settings-general-write-every-setting-station-radios = 将此电台站点的所有设置——电台、模式、服务器、记忆信道、频段计划——写入一个文件，可以带到另一套安装中使用。你的日志和任何已保存的服务器密码仍保留在此。
settings-general-import = 导入……
settings-general-replace-station-s-settings-ones = 用从另一套安装导出的文件中的设置，替换此电台站点的设置。之后请重启 sdroxide。
settings-general-import-overwrites-what-here-file = 导入会按文件逐个覆盖此处的内容，并在 sdroxide 下次启动时生效——否则内存中的现有设置会把它再原样写回。文件中未提及的内容保持不变，因此来自单电台站点的配置包不会删除此处的第二台电台。你的日志永远不在该文件中：如果也要迁移日志，请在日志窗口中以 ADIF 格式导出。
settings-general-sdroxide-settings = sdroxide 设置
settings-general-swr-guard = 驻波比保护
settings-general-stop-transmitting-high-swr = 驻波比过高时停止发射
settings-general-trip = 触发阈值
settings-general-stops-transmission-when-radio-reports =
    当电台报告的驻波比达到或超过此数值时，停止发射，并在你确认之前锁定发射功能。可捕获天线断开、馈线故障或开关误接到错误端口等情况。

    调谐则区别对待，因为馈入失配正是调谐的目的：天线调谐器可获得两倍的阈值，并且在保护开始生效之前有约五秒的时间。手动调谐器如果耗时超过这一时长，则需要在本次会话中关闭保护。

    需要通过 CAT 报告驻波比的电台。忽略每次发射的前五分之一秒，并且不等待高功率。
settings-general-per-mode-settings = 按模式设置
settings-general-reset-every-mode = 重置所有模式
settings-general-forget-every-mode-s-remembered = 清除每种模式记住的 AGC、静噪、降噪、陷波和立体声开关，并把每种模式自己的默认值恢复回来。
settings-general-selecting-mode-lays-mode-s = 选择某种模式时，接收机会应用该模式自己的 AGC、静噪、降噪、陷波和立体声开关的起始值；更改其中一项，则只为该模式记住。接收机滤波器/降噪行末尾的圆形箭头按钮会列出已更改的项目，并把当前模式恢复为默认值。
settings-general-your-audio-speakers-microphone = 你的音频（扬声器 / 话筒）
settings-general-output = 输出
settings-general-channels = 声道
settings-general-which-speaker-channel-s-received = 接收的音频送往哪个扬声器声道。选择左或右，会把所有内容（混合为单声道）只送到该侧。
settings-general-input = 输入

## 设置窗口，网络服务（src/app/settings/net.rs）

settings-net-operator = 操作员
settings-net-callsign-grid-are-set-general = 呼号和网格定位在“常规”选项卡中设置。
settings-net-set-your-callsign-grid-general = ⚠ 请在“常规”选项卡中设置你的呼号和网格定位。
settings-net-freedv-reporter = FreeDV Reporter
settings-net-enable = 启用
settings-net-connects-whenever-enabled-your-station = 启用时随时连接。只有当电台处于 RADE 模式时，你的电台才会显示给其他人；在其他任何模式下，你保持连接但处于隐藏状态。
settings-net-station = 电台
settings-net-message = 消息
settings-net-receive-only-i-cannot-transmit = 仅接收（我无法发射）
settings-net-server = 服务器
settings-net-host = 主机
settings-net-port = 端口
settings-net-tls-wss = TLS（wss://）
settings-net-not-yet-implemented-freedv-gui = 尚未实现——FreeDV 图形界面也使用普通的 ws://。
settings-net-reporting = 报告
settings-net-report-stations-i-decode = 报告我解码的电台
settings-net-sends-rx-report-each-callsign = 对从 RADE 帧尾（End-of-Over）中恢复的每个呼号，发送一条 rx_report。
settings-net-show-other-reporter-stations-spots = 将其他报告站显示为信号
settings-net-adds-them-panadapter-overlay-world = 在 FREEDV 过滤器下，把它们加入全景图叠加、世界地图和 SPOTS 窗口。
settings-net-set-your-callsign-grid-general-2 = ⚠ 请在“常规”选项卡中设置你的呼号和网格定位。两者缺一，连接就只能查看：你能看到其他电台，但自己不会出现在列表中。
settings-net-apply = {" "}应用{" "}
settings-net-persist-re-connect = 保存并（重新）连接
settings-net-longwave-shortwave-stations-labelled-waterfall = 瀑布图上标注的长波和短波电台。短波时刻表会在当前广播季节从 eibispace.de 下载，并在季节变更时重新下载。
settings-net-season = 季节
settings-net-downloading = 正在下载……
settings-net-using-built-copy = 正在使用内置副本
settings-net-your-own-stations-corrections-go = 你自己的电台和修正内容写在此文件中。sdroxide 从不写入它，此处的条目会替换同名且同频率的预定条目。
settings-net-reload = 重新加载
settings-net-re-read-your-station-file = 编辑后重新读取你的电台文件
settings-net-download-schedule-now = 立即下载时刻表
settings-net-fetch-season-s-schedule-again = 重新获取本季的时刻表，替换缓存副本
settings-net-broadcast-stations-labelled-waterfall-come = 瀑布图上标注的广播电台来自本版本内置的列表。编辑它们需要使用桌面应用。
settings-net-ask-service-whether-these-credentials = 询问服务这些凭据是否有效。会先应用上面的设置，并且不记录任何内容：它只读取。
settings-net-checking = 正在检查……

## 设置窗口：带有数值填充的消息。

settings-amp-connected-to = 已连接到 { $model }
settings-controls-last-midi = 最后一条消息：{ $message }  数值 { $value }
settings-general-band-plan-file = 每个频段边界和子频段都来自 { $path }。该文件首次运行时由内置的 IARU 表写入，之后即归你编辑——把某个频段缩窄到你的执照范围，sdroxide 就会拒绝在其之外发射。频率以 MHz 为单位；删除该文件可重新获得默认值的副本。这是区域分配，而不是你的执照：你自己的条件可能更窄，而且同一区域内的各国计划也有所不同。
settings-general-exported = 已导出 { $note }——请选择保存位置。
settings-general-export-failed = 导出失败：{ $error }
settings-general-import-failed = 导入失败：{ $error }
settings-general-imported = { $summary }——请重启 sdroxide 以使用它们。
settings-general-import-skipped = 已跳过 { $path }：{ $why }
settings-general-swr-tripped = ⚠ 当前已在 { $swr }:1 时触发——发射已被锁定。
settings-net-call-grid-set-on-general = { $call } / { $grid } ——在“常规”选项卡中设置
settings-net-reporting-as = 以 { $call } / { $grid } 身份报告——SDRoxide { $version }
settings-net-schedule-download-failed = ⚠ 下载失败：{ $error }。正在使用内置时刻表。
settings-net-test-service = 测试 { $service }

## 设置窗口（src/app/settings/mod.rs），续

settings-transmit-off-while-converter-set = 设置了变频器时发射已关闭——请说明上面发射线路中的内容，以重新开启发射。
settings-transmit-goes-through-same-converter = 发射经过同一变频器，因此电台在两个方向上都以距表盘相同的偏移工作。
settings-transmit-not-converted-radio-transmits = 发射未经转换：电台在表盘所示频率上发射，而接收则经过变频器。
settings-transmit-takes-own-offset-separate = 发射使用其自己的偏移，与接收偏移分开。
settings-no-enabled-subscriptions-update = 没有可更新的已启用订阅。
settings-add-radio-here-station-you = 添加电台——在此处，或在你已连接的某个电台站点上

## 设置窗口，常规，续

settings-general-every-band-edge-sub-segment = 每个频段边界和子频段都来自电台所连接机器上的 bandplan.json。这是区域分配，而不是你的执照：你自己的条件可能更窄，而且同一区域内的各国计划也有所不同。

## 设置窗口，图片上传（src/app/settings/picup.rs）

settings-picup-received-pictures-ftp = 接收的图片 → FTP
settings-picup-uploads-every-received-sstv-picture = 在每张接收到的 SSTV 图片保存时将其上传（如勾选，也包括每张天气图），布局与 QSSTV 相同，因此为 QSSTV 编写的网页无需修改即可使用。
settings-picup-upload-received-pictures = 上传接收的图片
settings-picup-server = 服务器
settings-picup-port = 端口
settings-picup-security = 安全性
settings-picup-user = 用户
settings-picup-password = 密码
settings-picup-certificate = 证书
settings-picup-trusted-when-computer-s-operating = 当本计算机的操作系统信任它时即被信任，与此处的浏览器相同。可在下方添加私有 CA（PEM）。
settings-picup-extra-ca = 额外 CA
settings-picup-only-certificate-fingerprint-accepted-way = 只接受具有此指纹的证书——这是信任自签名服务器的方式。“测试”会显示服务器的指纹。
settings-picup-any-certificate-accepted-link-encrypted = ⚠ 接受任何证书。连接是加密的，但路径上的任何人都可以冒充服务器并读取密码。
settings-picup-plain-ftp-sends-password-pictures = ⚠ 普通 FTP 以未加密方式发送密码和图片。
settings-picup-sstv = SSTV
settings-picup-sstv-dir = SSTV 目录
settings-picup-weather-fax = 天气传真
settings-picup-wefax-dir = WEFAX 目录
settings-picup-empty-login-directory = 留空则为登录目录。
settings-picup-file-names = 文件名
settings-picup-format = 格式
settings-picup-add-extension = 添加扩展名
settings-picup-image1-png-rather-than-qsstv = 使用 image1.png，而不是 QSSTV 的无扩展名 image1
settings-picup-caption-bar = 标题栏
settings-picup-time-frequency-mode-sender-s = 顶部显示时间、频率、模式以及发送方的 FSK ID，与 QSSTV 的绘制方式相同。仅已上传的副本带有标记。
settings-picup-fit-960-768 = 适配为 960 × 768
settings-picup-qsstv-s-heartbeat-tiny-picture = QSSTV 的心跳：一张小图片，其高度表示当前分钟，每张图片旁边都会上传一张，程序关闭时宽度变为两个像素。
settings-picup-nat-workaround = NAT 变通
settings-picup-ignore-private-address-server-s = 忽略服务器被动模式回复中的私有地址，改用服务器自己的地址——适用于位于 NAT 之后、不知道自己公网地址的服务器。
settings-picup-apply = {" "}应用{" "}
settings-picup-test = 测试
settings-picup-connect-settings-above-applied-not = 使用上面的设置连接（无论是否已应用）、登录，并在每个上传目录中写入和删除一个小文件。
settings-picup-checking = 正在检查……
settings-picup-trust-certificate = 信任此证书
settings-picup-pin-switch-pinned-fingerprint-one = 固定它：将开关切换为“固定指纹”并填入此指纹。请先与服务器核对——这是服务器所报告的内容，并不能证明其身份。然后点击“应用”。
settings-picup-last-upload = 最近一次上传

## 设置窗口，配置文件（src/app/settings/profiles.rs）

settings-profiles-profiles-station-s-saved-setups = 配置文件：电台站点保存的设置
settings-profiles-profile-captures-whole-working-setup = 配置文件会记录整套工作设置——表盘和 VFO、模式和滤波器、增益、驱动和天线、你的呼号和消息模板，以及分频段的寄存器——并可一键恢复。硬件（后端、音频设备、变频器）则被有意排除在外：配置文件记录的是你操作电台的方式，而不是电台本身。
settings-profiles-save-current-setup = 保存当前设置
settings-profiles-profile-name = 配置文件名称
settings-profiles-save = 保存
settings-profiles-applies-whatever-sdroxide-attached-dials = 适用于 sdroxide 所连接的任何电台：表盘和 VFO、模式和滤波器、电平、增益、驱动、天线、数字身份以及频段堆栈。
settings-profiles-no-profiles-saved-yet-name = 尚未保存任何配置文件——请在上面命名一个，然后点击“保存”。
settings-profiles-saved-profiles = 已保存的配置文件
settings-profiles-apply = 应用

## 设置窗口，收发开关（src/app/settings/relay.rs）

settings-relay-transmit-receive-switch = 收发开关
settings-relay-waiting-station-s-t-r = 正在等待电台站点的收发开关配置……
settings-relay-closes-contact-while-station-transmits = 在此电台站点发射期间闭合一个触点——用于断开并接地 SDR 的天线输入，并与之按顺序驱动功放或外置收发继电器。适用于廉价的 USB 继电器板（LCUS、KMtronic、Numato），通过串口 RTS/DTR 线路可用于任何需要 PTT 闭合的接口，也可通过命令钩子用于其他任何设备。
settings-relay-hardware = 硬件
settings-relay-board = 板卡
settings-relay-lcus-four-byte-a0-command = LCUS 是以十几个名称出售的四字节 A0 命令板；KMtronic 和 Numato 也可以查询其当前设置
settings-relay-port = 端口
settings-relay-choose = — 请选择 —
settings-relay-device = 设备
settings-relay-gpio-pin = GPIO 引脚
settings-relay-pin-3-every-homebrew-plan = 第 3 脚见于所有自制方案和所有商用板卡——它位于封装的末端，便于焊接导线。如果继电器没有咔哒声，说明该卡是不支持这些引脚的仿制品。
settings-relay-gpio-chip = GPIO 芯片
settings-relay-linux-only-gpiodetect-lists-chips = 仅限 Linux。`gpiodetect` 可列出本机上的芯片。
settings-relay-transmit = 发射时
settings-relay-receive = 接收时
settings-relay-nothing-switched = 未切换任何内容。
settings-relay-starting-program-costs-unpredictable-amount = 启动一个程序所需的时间不可预测，而且这段时间都不计入提前量——因此在此处应比对继电器板预留更宽的余量。触点表中的数值对此链路没有意义：你的命令会在按下时运行一次，松开时再运行一次。
settings-relay-contacts = 触点
settings-relay-contact-1-rts-contact-2 = 触点 1 为 RTS，触点 2 为 DTR。
settings-relay-no = 编号
settings-relay-gpio = GPIO
settings-relay-name = 名称
settings-relay-job = 功能
settings-relay-tx-closes = 发射时闭合
settings-relay-lead = 提前
settings-relay-hold = 保持
settings-relay-line-s-offset-chip-bcm = 此芯片上的线路偏移——在树莓派针脚排针上即为 BCM 编号，而非物理针脚号
settings-relay-sdr-antenna = SDR 天线
settings-relay-decides-default-timings-what-log = 决定默认时序，以及日志如何称呼它——对于波段解码器，还决定该触点是否跟随下方的频段表
settings-relay-transmitting-energises-coil-off-transmitting =
    开：发射时线圈通电。关：发射时线圈释放。

    这首先是接线决定，然后才是设置——见下面的说明。
settings-relay-how-long-before-rf-contact = 在射频信号出现之前多久，此触点从其频段的接收字切换到发射字。重新调谐时会立即切换；此项用于按下发射的时刻，因为仅发射滤波器必须在功率到达之前接入电路。
settings-relay-closed-long-before-rf-small = 在射频之前闭合的时长。小型同轴继电器需要 5 至 15 毫秒吸合。发射会等待其中最长的时间，因此数值较大会在接收音频中产生可听见的间隙；超过 250 毫秒的数值将被忽略。
settings-relay-how-long-after-rf-stops = 射频停止之后多久，此触点从其发射字切回接收字。
settings-relay-opened-long-after-rf-stops = 射频停止之后的这段时间后断开。有意比提前量更长：天线稍晚切回没有代价，而过早切回则会损坏前端。零是一个有效的答案，也是功放按键线路的正确设置。
settings-relay-test = {" "}测试{" "}
settings-relay-not-while-station-air = 电台在空中时不可用
settings-relay-close-contact-half-second-so = 将此触点闭合半秒，以便你听到继电器动作，并在发射机断电的情况下检查接线
settings-relay-remove = 删除
settings-relay-add-contact = + 添加触点
settings-relay-if-will-not-answer = 如果它不响应
settings-relay-switch-exists-protect-receiver-not = 一个用来保护接收机的开关，如果它悄悄停止了保护，就没有存在的价值——因此默认是拒绝。若一根松动的 USB 线缆结束了你的比赛，那属于更糟的结果，则可以选择另一项。
settings-relay-order-timings-longest-lead-closes = 顺序由时序决定：提前量最长的先闭合，保持时间最长的最后断开。因此天线继电器应设为较长的提前量和较长的保持时间，而功放则各取较短的值——这样天线总是先于功放按键被切换，而功放总是先于天线复位而解除按键。
settings-relay-transmit-sense-input = 发射检测输入
settings-relay-when-transceiver-keys-itself-own = 当收发信机自行按下发射时——例如其自带的话筒按钮、脚踏开关、VOX 或键控器——sdroxide 会通过 CAT 询问得知，而该回答要在通话开始后几百毫秒才到达。若将电台的 SEND 线路经光耦接入同一端口的握手输入，则几毫秒内即可察觉。这样一来，表头和发射互锁都会更快。
settings-relay-line = 线路
settings-relay-transmitting = 发射时为
settings-relay-high-line = 高电平
settings-relay-most-opto-isolated-interfaces-pull = 大多数光隔离接口会在电台按键时把线路拉低，因此通常应关闭此项
settings-relay-belongs-radio = 所属电台
settings-relay-which-radio-tab-sensed-transceiver = 被检测的收发信机属于哪个电台选项卡。在只有一台电台的站点上，此项为 1，且不影响任何内容。
settings-relay-needs-serial-port-handshake-lines = 需要串口——握手线路位于其上。
settings-relay-wiring-only-fail-safe-survives = 只有接线才是能够在本程序失效后仍然有效的故障保护。
settings-relay-choose-tx-closes-so-de = 选择“发射时闭合”，使得*失电*的触点恰是你所需的状态：当板卡被拔掉、计算机关机或 sdroxide 未运行时。若接线使失电即接地，则继电器失效时接收机会失聪——令人烦恼，但安全。若接线方式相反，继电器失效加上一次发射，前端就会损坏。若其后接的是真正的功放，请选择前一种。
settings-relay-cannot-protect-first-instant-over = 对于你在电台上开始的通话，它无法保护最初的瞬间。若从 sdroxide 按下发射，触点总是先于射频动作；若接入上面的检测输入，它们则会在几毫秒内跟随。若前端确实值得保护，还应另外使用由射频检测的硬件收发开关——计算机上的任何程序都不能单独解决全部问题。
settings-relay-contacts-transmit = 发射时的触点
settings-relay-not-open = 未断开。
settings-relay-status-unknown-press-apply = 状态未知——请按“应用”。
settings-relay-station-air-apply-will-take = 电台在空中——“应用”将在本次通话结束后生效。
settings-relay-band-decoder-outputs = 频段解码器输出
settings-relay-one-control-word-per-band = 每个频段一个控制字：位 0 为触点 1，位 1 为触点 2，依此类推，与上面的编号一致。接收跟随接收表盘所在的频段；发射则跟随按下发射的那台电台的频段——在射频之前经每个触点的提前量接入，并在保持时间之后切回。对于不关心射频方向的滤波器，两者设为相同的值；对于仅接收前置放大器的旁路或仅发射低通滤波器组，则设为不同的值。保持为 0 的频段不输出任何信号。在“应用 / 重新连接”后生效。
settings-relay-band = 频段
settings-relay-rx = 接收
settings-relay-tx = 发射
settings-relay-outputs-asserted = 已输出的信号
settings-relay-other = 其他
settings-relay-everywhere-outside-amateur-bands-short = 业余频段之外的所有位置——短波收听，以及变频器表盘落到任何频段都未覆盖之处。
settings-relay-apply = {" "}应用{" "}
settings-relay-persist-re-open-switch = 保存并（重新）打开开关

## 设置窗口，远程（src/app/settings/remote.rs）

settings-remote-connect-server = 连接服务器
settings-remote-drive-sdroxide-running-somewhere-else = 通过网络操作在别处运行的 sdroxide——机房计算机、远程站点。请在那里以 --server 启动它，并在此输入其地址。电台、解码器和日志仍保留在该机器上；此屏幕获取瀑布图和音频，并使用你在此处的扬声器和话筒。
settings-remote-address = 地址
settings-remote-host-name-ip = 主机名或 IP
settings-remote-machine-running-sdroxide-server-host = 运行 sdroxide --server 的机器：可以是主机名、IPv4 或 IPv6 地址，或者在服务器位于反向代理之后时使用完整的 ws://（或 wss://）URL——URL 会按原样使用，忽略下面的端口和开关。
settings-remote-port = 端口
settings-remote-port-server-listens-server-port = 该服务器监听的端口——即其 config.toml 中的 server_port。除非指定了 --port，否则为 4950。
settings-remote-secure = 安全
settings-remote-wss-tls = wss://（TLS）
settings-remote-tick-where-server-sits-behind = 当服务器位于一个为其终止 HTTPS 的组件之后时勾选此项——例如端口 443 上的反向代理，这是通过公共互联网访问电台的常见方式。sdroxide 自身的服务器使用普通的 ws://，不需要勾选此项。
settings-remote-connect = {" "}连接{" "}
settings-remote-open-station-radio-tab-own = 将此电台站点打开为其自己的电台选项卡
settings-remote-client-cannot-hold-second-connection = 此客户端无法保持第二个连接。
settings-remote-station-appears-tab-beside-radios =
    该电台站点会作为一个选项卡出现在此处已打开的电台旁边，窗口顶部的选项卡栏可在它们之间切换（⊞ 可将两者并排显示）。在电台选项卡顶部的列表中关闭它即可断开连接；服务器上的任何内容都不会更改。

    如果服务器要求输入用户名和密码，其登录界面会出现在新选项卡中。普通的 ws:// 链接不会对这些内容加密，因此通过公共互联网时，请使用 VPN 或 SSH 隧道，或在服务器前端终止 HTTPS 并勾选上面的“安全”。

## 设置窗口，服务器（src/app/settings/servers.rs）

settings-servers-wsjt-x-udp-broadcast = WSJT-X UDP 广播
settings-servers-waiting-station-s-wsjt-x = 正在等待电台站点的 WSJT-X 广播配置……
settings-servers-sends-decodes-station-status-logged = 以 WSJT-X 的方式发送解码结果、电台状态和已记录的通联，因此 GridTracker、JTAlert、N1MM+ 和 Log4OM 无需修改即可与 sdroxide 配合使用。仅用于输出——此套接字上的任何内容都无法控制电台。
settings-servers-enable = 启用
settings-servers-send = 发送至
settings-servers-t-127-0-0-1-reaches = 127.0.0.1 可到达本机上的客户端；局域网地址或组播组（224.0.0.1）可到达其他设备
settings-servers-port = 端口
settings-servers-t-2237-port-every-client-defaults = 2237 是每个客户端的默认端口
settings-servers-identify = 标识为
settings-servers-name-clients-see-some-loggers = 客户端看到的名称。某些日志软件只接受自称为 WSJT-X 的流量。
settings-servers-n1mm-contactinfo-broadcast = N1MM+ contactinfo 广播
settings-servers-same-news-n1mm-s-words = 以 N1MM 的格式发布相同的信息：每条已记录的通联一个 XML 数据报，供那些监听 N1MM 而非 WSJT-X 的日志软件使用——世界无线电联盟（World Radio League）的桥接程序两者都支持。仅包含通联；此协议中没有解码数据流。它有自己的开关和自己的端口，因为一个电台站点可能两者都需要。
settings-servers-t-127-0-0-1-reaches-2 = 127.0.0.1 可到达本机上的客户端。N1MM 对比赛网络的建议是该子网的广播地址——对于 192.168.1.n 网络即为 192.168.1.255——它可以同时到达每个操作位置。
settings-servers-t-12060-port-n1mm-s-documentation = 12060 是 N1MM 文档推荐的端口
settings-servers-station-name = 电台站点名称
settings-servers-what-n1mm-calls-stationname-name = N1MM 所称的 StationName：发送该数据包的计算机名称。日志软件借此区分不同的操作位置。
settings-servers-decoded-cw-over-udp = 通过 UDP 发送解码的 CW
settings-servers-cw-panel-s-copy-plain = 将 CW 面板的文字以纯文本形式发送，每段稳定后作为一个 UDP 数据报发送——用于电台站点控制器、日志软件的 CW 窗口或脚本。没有协议：监听方只需追加接收到的内容（可尝试 nc -ul 9999）。只发送已确定的文字，从不发送解码器仍在修正的尾部。每台电台有各自的开关。
settings-servers-t-127-0-0-1-reaches-3 = 127.0.0.1 可到达本机上的程序；局域网地址或该子网的广播地址可到达其他设备。
settings-servers-t-9999-default = 默认为 9999
settings-servers-apply = {" "}应用{" "}
settings-servers-persist-re-open-broadcast-sockets = 保存并（重新）打开广播套接字
settings-servers-hamlib-rigctld-server = Hamlib rigctld 服务器
settings-servers-waiting-station-s-rigctld-configuration = 正在等待电台站点的 rigctld 配置……
settings-servers-listen = 监听地址
settings-servers-t-127-0-0-1-machine = 127.0.0.1（本机）
settings-servers-t-0-0-0-0-whole = 0.0.0.0（整个网络）
settings-servers-t-4532-port-every-rigctld-client = 4532 是每个 rigctld 客户端默认采用的端口
settings-servers-rig-name = 电台名称
settings-servers-reported-clients = 报告给客户端的名称
settings-servers-max-clients = 最大客户端数
settings-servers-allow-clients-transmit = 允许客户端发射
settings-servers-off-refuses-every-key-request = 关闭后会拒绝每一个按键请求，并停止宣告发射范围，因此 Hamlib 本身会拒绝按键。
settings-servers-rigctld-has-no-authentication-0 = ⚠ rigctld 没有身份验证。若监听 0.0.0.0，任何能访问该端口的人都可以调谐并按下你的发射机。
settings-servers-t-1-client = 1 个客户端
settings-servers-not-running = 未运行。
settings-servers-status-unknown-press-apply = 状态未知——请按“应用”。
settings-servers-persist-re-bind-server = 保存并（重新）绑定服务器
settings-servers-lets-any-hamlib-capable-program = 允许任何支持 Hamlib 的程序操作此电台：频率、模式、PTT、异频、RIT 和功率。在 WSJT-X、fldigi 或 CQRLOG 中，选择电台“Hamlib NET rigctl”（型号 2）并将其指向此地址；在 GPredict 和 N1MM 中，直接输入主机和端口。与 TCI 服务器不同，它只传输控制信息——没有音频，也没有 IQ。
settings-servers-antenna-rotator = 天线旋转器
settings-servers-waiting-station-s-rotator-configuration = 正在等待电台站点的旋转器配置……
settings-servers-points-motorized-antenna-satellite-you = 通过 Hamlib rotctld 守护进程，将电动天线指向你所锁定的卫星（例如 GS-232B 旋转器可运行“rotctld -m 603 -r /dev/ttyUSB0”，若要在没有硬件的情况下试用则运行“rotctld -m 1”）——一个守护进程即可覆盖 Hamlib 驱动的每一台旋转器——或通过 PSTRotator，在其设置中启用 UDP Control。
settings-servers-program = 程序
settings-servers-host = 主机
settings-servers-t-4533-rotctld-s-default-pstrotator = 4533 是 rotctld 的默认端口。PSTRotator 的 UDP 控制端口默认为 12000，并在下一个端口 12001 上回应位置查询——该端口必须在本机上空闲。
settings-servers-min-elevation = 最低仰角
settings-servers-below-rotator-parks-instead-tracking = 低于此仰角时，旋转器会停靠而不是跟踪——请将其设为你所在地的地平线或屋顶线
settings-servers-azimuth-offset = 方位偏移
settings-servers-added-every-command-rotator-whose = 加入每条命令中，适用于北向有偏差的旋转器
settings-servers-min-movement = 最小移动量
settings-servers-steps-smaller-than-are-not = 小于此值的步进不会发送——追逐十分之一度的移动只会白白磨损电机
settings-servers-park-position = 停靠位置
settings-servers-drive-position-when-idle = 空闲时转到一个位置
settings-servers-off-leaves-antenna-wherever-last = 关闭后，天线停留在上一次过境结束时的位置
settings-servers-not-connected = 未连接。
settings-servers-persist-re-connect-daemon = 保存并（重新）连接守护进程
settings-servers-waiting-station-s-tci-server = 正在等待电台站点的 TCI 服务器配置……
settings-servers-built-tci-server = 内置 TCI 服务器
settings-servers-device-name = 设备名称
settings-servers-off-leaves-control-receive-streams = 关闭后，控制和接收数据流仍然工作，但所有按键请求都会被拒绝。
settings-servers-tci-has-no-authentication-0 = ⚠ TCI 没有身份验证。若监听 0.0.0.0，任何能访问该端口的人都可以调谐并按下你的发射机。
settings-servers-tci-client = TCI 客户端
settings-servers-when-you-tune-send-radio = 调谐时，将此电台的 VFO 发送到另一台 TCI 服务器，使该电台随之调谐。仅控制——没有音频，没有 IQ，也不会按下发射机。
settings-servers-server = 服务器
settings-servers-same-address-server-above-client = 这与上面服务器的地址相同。只有当该服务器不是正在监听的那一个时，客户端才会连接——当 590Ctrl 正在使用此端口时，请将其关闭，或改用其他端口。
settings-servers-tci-client-connected = ● TCI 客户端已连接
settings-servers-tci-client-connecting = TCI 客户端正在连接……
settings-servers-tci-client-press-apply = TCI 客户端——请按“应用”。
settings-servers-persist-re-bind-server-connect = 保存并（重新）绑定服务器，并连接 TCI 客户端
settings-servers-server-lets-tci-capable-programs = 服务器允许支持 TCI 的程序操作此电台：频率和模式控制、宽带 IQ、接收音频和发射音频。在 WSJT-X 中，选择电台类型 SunSDR (TCI)，将其指向此地址，并把 PTT 设为 TCI。TCI 客户端则是反方向：sdroxide 的表盘会以 VFO 命令的形式，发送到上面地址处的 590Ctrl（或任何 TCI 服务器）。它不传输音频。

## 设置窗口，TLE（src/app/settings/tle.rs）

settings-tle-satellites-element-sets-frequencies = 卫星：轨道根数与频率
settings-tle-waiting-station-s-satellite-configuration = 正在等待电台站点的卫星配置……
settings-tle-tracker-already-fetches-celestrak-s = 跟踪器本身已会获取 CelesTrak 的业余卫星组。此处用于其他一切：NOAA 气象卫星、尚未进入该组的新立方星，或比已收到的版本更新的轨道根数。这些列表在电台引擎运行的地方获取并保存，因此无论在哪里打开此应用，卫星集合都相同。
settings-tle-subscriptions = 订阅
settings-tle-listings-fetched-kept-current-same = 列表会获取并保持更新，更新周期与业余卫星集相同，即每六小时一次。太阳窗口打开时以及在此处点击“立即更新”时刷新。
settings-tle-fetch-track-listing = 获取并跟踪此列表
settings-tle-remove-subscription = 删除此订阅
settings-tle-orbits = 轨道
settings-tle-which-satellites-listing-get-orbit = 此列表中哪些卫星会获得轨道环和标签。整组卫星需要选择“精选”：同时显示九十个轨道环会无法辨认，而一个都不显示则只会留下九十个无名圆点。
settings-tle-nothing-listing-sdroxide-s-curated = 此列表中的任何卫星都不在 sdroxide 的精选列表中——该列表只有十颗业余卫星，因此这里的行为将与“无”完全相同。
settings-tle-all-satellites = 所有卫星
settings-tle-catalogue-numbers-keep-comma-separated = 要保留的编目号，用逗号分隔。留空则跟踪列表中包含的全部卫星。
settings-tle-not-fetched-yet = 尚未获取
settings-tle-subscription = + 订阅
settings-tle-update-now = {" "}立即更新{" "}
settings-tle-ask-radio-engine-fetch-every = 要求电台引擎立即获取每个已启用的订阅
settings-tle-celestrak-groups = CelesTrak 分组
settings-tle-pasted-element-sets = 粘贴的轨道根数
settings-tle-one-off-these-do-not = 用于一次性使用。这些不会自动更新，而且 SGP4 在轨道根数超过其历元两周后就会停止推算——凡是要长期使用的，请改为订阅。
settings-tle-track-one = 跟踪此项
settings-tle-show-two-element-lines = 显示两行轨道根数
settings-tle-remove = 删除
settings-tle-paste-two-three-line-element = 在此粘贴两行或三行的轨道根数
settings-tle-add-pasted = + 添加粘贴内容
settings-tle-nothing-paste-box-looked-like = 粘贴框中的内容都不像是轨道根数。
settings-tle-clear-box = 清空文本框
settings-tle-frequencies = 频率
settings-tle-shown-under-pass-table-solar = 显示在太阳窗口的过境表下方。此处的条目会完全替换该编目号内置的条目，因此除非你打算丢弃其余内容，否则请从它的副本开始修改。
settings-tle-remove-satellite-s-entry = 删除此卫星的条目
settings-tle-link = 链路
settings-tle-downlink = 下行
settings-tle-uplink = 上行
settings-tle-mode = 模式
settings-tle-note = 备注
settings-tle-fm-repeater = FM 中继
settings-tle-fm = FM
settings-tle-ctcss-67-0-hz = CTCSS 67.0 Hz
settings-tle-link-2 = + 链路
settings-tle-copy-built = 复制内置项
settings-tle-frequency-either-one-number-145 = 频率可以是一个数字（145.800），也可以是写作 145.950-145.970 的转发器通带。信标的方向留空即可。
settings-tle-norad = NORAD
settings-tle-satellite = + 卫星
settings-tle-frequency-entry-needs-satellite-s = 频率条目需要卫星的 NORAD 编目号。

## 设置窗口，Wavelog（src/app/settings/wavelog.rs）

settings-wavelog-address = 地址
settings-wavelog-api-key = API 密钥
settings-wavelog-your-wavelog-s-front-page = 你的 Wavelog 首页地址——https://log.example.org 或 http://192.168.1.10/wavelog——以及来自其 API 页面的密钥。上传需要读写密钥。
settings-wavelog-plain-http-address-internet-api = 互联网上某地址的普通 http:// 连接：API 密钥会以未加密方式传输。在你自己的网络内没有问题；跨互联网时请使用 https://。
settings-wavelog-trusted-cert = 受信任的证书
settings-wavelog-forget-certificate = 忘记此证书
settings-wavelog-go-back-ordinary-checking-against = 恢复为依据公共证书颁发机构的常规检查——适用于现在已拥有正式证书的服务器。
settings-wavelog-server-presented-certificate-sha-256 = 服务器提供的证书（SHA-256）：
settings-wavelog-compare-your-server-s-e = 将其与你服务器的证书进行比较——例如 `openssl x509 -noout -fingerprint -sha256 -in cert.pem`——只有两者一致时才信任它。此后将只接受这个确切的证书，其他任何证书都会被拒绝。
settings-wavelog-trust-certificate = 信任此证书
settings-wavelog-station-location = 电台站点位置
settings-wavelog-none = — 无 —
settings-wavelog-press-test-above-list-station = 点击上面的“测试”，列出你的 Wavelog 中定义的电台站点位置。
settings-wavelog-default = 默认
settings-wavelog-where-qsos-go-when-operating = 当操作配置文件未指定位置时，通联记录到哪里。
settings-wavelog-profile = 此配置文件
settings-wavelog-use-default = 使用默认值
settings-wavelog-not-set = 未设置
settings-wavelog-no-station-location-chosen-uploads = 未选择电台站点位置——在选择之前，上传都会失败。
settings-wavelog-qsos-not-wavelog = 不在 Wavelog 中的通联
settings-wavelog-every-qso-logbook-wavelog = 日志中的每一次通联都已在 Wavelog 中。
settings-wavelog-everything-logbook-wavelog-has-not = 日志中所有 Wavelog 尚未确认的内容——包括自动上传开启之前的通联。Wavelog 已有的那些会作为重复项返回，并仅被标记为已发送。
settings-wavelog-set-address-key-station-location = 请先设置地址、密钥和电台站点位置。
settings-wavelog-yes-send-them = 是，发送它们
settings-wavelog-cancel = 取消
settings-wavelog-download-into-logbook = 下载到此日志
settings-wavelog-downloading-runs-where-radio-attached = 下载在电台所连接的机器上运行，并写入该机器的日志——请在那里设置。
settings-wavelog-download-qsos-start-every = 启动时以及每隔多久下载通联
settings-wavelog-downloaded-qsos-join-logbook-so = 下载的通联会加入日志，因此 FT8 和信号列表能知道你已通过其他程序和电台通联过哪些。每次下载只获取新增的内容。
settings-wavelog-press-test-above-choose-which = 点击上面的“测试”，选择要下载哪些电台站点位置。
settings-wavelog-sync-now = 立即同步
settings-wavelog-fetch-what-wavelog-has-added = 获取自上次下载以来 Wavelog 新增的内容。会先应用上面的设置。
settings-wavelog-full-resync = 完全重新同步……
settings-wavelog-download-ticked-locations-whole-compare = 完整下载已勾选的位置，并与日志比较：在 Wavelog 中编辑或删除的通联，或来自已取消勾选位置的通联，都会被同步一致。在任何操作发生之前，你会先看到这些更改。
settings-wavelog-full-resync-changes-make = 完全重新同步——将要进行的更改
settings-wavelog-qsos-logged-here-imported-file = 在此记录或从文件导入的通联永远不会被更改或删除——只有从 Wavelog 下载的通联会如此。
settings-wavelog-apply = 应用
settings-wavelog-discard = 放弃
settings-wavelog-live-radio-status = 电台实时状态
settings-wavelog-send-frequency-mode-wavelog-they = 频率和模式变化时即发送到 Wavelog
settings-wavelog-radio-name = 电台名称
settings-wavelog-pick-radio-wavelog-s-live = 在 Wavelog 的实时记录中选择此电台，其表单会跟随表盘。需要读写密钥。仅限电台站点的主电台。

## 设置窗口，图片上传：带数值填充的消息

settings-picup-server-fingerprint = 服务器证书 SHA-256：{ $fingerprint }

## 设置窗口，配置文件：带数值填充的消息

settings-profiles-apply-hint = 将电台站点恢复为“{ $name }”：表盘、VFO、模式、滤波器、增益、驱动、天线、身份和频段堆栈。
settings-profiles-delete = 删除“{ $name }”

## 设置窗口，收发开关：带数值填充的消息

settings-relay-not-usable = 尚不可用：{ $why }
settings-relay-receiving = 接收中
settings-relay-rx-tx-words = { $rx } / 发射 { $tx }

## 设置窗口，服务器：带数值填充的消息

settings-servers-n-clients = { $n } 个客户端
settings-servers-listening = ● 正在监听 { $addr } — { $clients }
settings-servers-failed = 失败：{ $error }
settings-servers-rotator-at = ● 已连接——天线方位 { $az }°，仰角 { $el }°
settings-servers-tci-client-error = TCI 客户端：{ $error }

## 设置窗口，TLE：带数值填充的消息

settings-tle-subscription-age = { $count } 颗卫星 · 已过 { $age }
settings-tle-subscribed = 已订阅 { $name }。请按“立即更新”获取。
settings-tle-norad-too-stale = NORAD { $norad } · 已过 { $age } — 过旧，无法推算
settings-tle-norad-age = NORAD { $norad } · 已过 { $age }
settings-tle-norad-n = NORAD { $norad }
settings-tle-added = 已添加 { $n } 组轨道根数。
settings-tle-added-refreshed = 已添加 { $added } 组，并刷新了 { $r } 组轨道根数。
settings-tle-links = { $n } 条链路
settings-tle-replace-links = 用 { $name } 的内置链路替换这些链路

## 设置窗口，Wavelog：带数值填充的消息

settings-wavelog-call-mismatch = 位置“{ $location }”的呼号为 { $station_call }，但你以 { $call } 记录通联——Wavelog 会拒绝这些通联。
settings-wavelog-grid-mismatch = 位置“{ $location }”位于 { $station_grid }，但你的网格定位为 { $grid } ——Wavelog 会拒绝这些通联。
settings-wavelog-location-n = 位置 #{ $id }
settings-wavelog-saved-with-callsign = 已与设置 → 配置文件中的呼号（{ $call }）一同保存，因此便携配置文件可以记录到便携位置。
settings-wavelog-owed = 自开启自动上传以来记录的 { $n } 条通联仍待发送——每 5 分钟重试一次。
settings-wavelog-send-unsent = 发送 { $n } 条尚未在 Wavelog 中的通联……
settings-wavelog-send-confirm = 将 { $n } 条通联发送到 { $location }？会先应用设置。
settings-wavelog-plan-add = • 将有 { $n } 条通联在 Wavelog 中被添加到日志
settings-wavelog-plan-update = • 有 { $n } 条已下载的通联在 Wavelog 中被更改，需要更新
settings-wavelog-plan-remove = • 有 { $n } 条已下载的通联不再在 Wavelog 中（或来自已取消勾选的位置），将被删除
settings-wavelog-plan-mark = • 将有 { $n } 条你自己的通联被标记为已在 Wavelog 中

## 设置窗口，电台选项卡（src/app/settings/radio.rs）

settings-radio-antenna = 天线
settings-radio-which-socket-back-radio-using =
    电台使用背面的哪个插座——即其自身的 ANT 命令，与前面板上 ANT 按钮的设置相同。

    立即生效，而且这是电台自己的设置，而非此处保存的副本：会话打开时会读回电台所在的插座。

    该选择按频段记忆。在 2 米切换到波束、在 40 米切换到线天线，则每次表盘进入相应频段时都会恢复——包括 sdroxide 下次启动时。此处保存的记忆信道也会带上其插座。
settings-radio-receive-antenna = 接收天线
settings-radio-radio-s-separate-receiving-antenna =
    电台独立的接收天线——背面的 RX ANT——切入或切出接收通路。这是它自己的设置，与电台上 RX ANT 按钮的设置相同。

    主插座上的天线无论如何都保留用于发射：这是一个额外的接收输入，例如环形天线或 Beverage 天线，而不是插座的选择。

    与上面的天线不同，此项*不*在此处记忆。电台自己按频段保存它，因此 sdroxide 会在每次频段变化后读回，并显示电台所报告的状态；只有勾选此框才会改变它。
settings-radio-receive-path = 位于接收通路中
settings-radio-radio-power = 电台电源
settings-radio-switch-radio-itself-off-back =
    通过控制链路，关闭电台本身再重新打开。这是本程序中唯一真正的开关——而非 sdroxide 自己的“连接”开关，后者只关闭 sdroxide 这一端，电台仍在运行。

    要让重新打开的指令能够生效，电台的控制端在关闭期间必须保持唤醒。通过网络时，这由“网络控制”实现。通过串口线缆时，则是电台的 CI-V 端口——对于在前面板关机而非拔掉电源的机器，它仍由市电供电；sdroxide 会在开机之前发送 Icom 自己文档要求的唤醒序列。

    关机会终止音频和表头——其背后已没有电台——而 sdroxide 会保持控制链路打开，以便重新开机的指令有地方可去。
settings-radio-on = 开
settings-radio-off = 关
settings-radio-waiting-configuration-machine-radio-attached = 正在等待电台所连接机器的配置。
settings-radio-sound-format = 声音格式
settings-radio-invert-spectrum = 反转频谱
settings-radio-swap-i-q = 交换 I/Q
settings-radio-mirror-panadapter-about-tuned-frequency =
    以调谐频率为轴镜像全景图，适用于声卡上 I 和 Q 相反的电台。明显的迹象是：瀑布图上充满了逼真的信号，但全都位于表盘的错误一侧，SSB 则出现在相反的边带上——在声卡上互换两根线缆同样可以解决此问题。

    仅接收：发射时电台接收到的是一个真正的音频信号，它没有可以反转的边带。
settings-radio-i-q-sample-rate = I/Q 采样率
settings-radio-how-fast-radio-s-i =
    电台的 I/Q 声卡以多快的速率运行——这也决定全景图显示多宽的频段，因为正交数据流覆盖其整个采样率：48 kHz 在表盘两侧各为 ±24 kHz，192 kHz 则为 ±96。

    最终由声卡决定。无法以此处所选速率运行的声卡，会以它能支持的最接近速率打开，全景图也会相应变窄——发生这种情况时，日志会在启动时说明。

    声卡速率越快，工作量也越大：机器清空它的时间相应减少，无法跟上的声卡会丢失采样。这表现为音频断断续续，而瀑布图看起来仍然完美，同样会记录在日志中。如果出现这种情况，请降低一档。
settings-radio-t-48-khz-24-khz = 48 kHz（±24 kHz）
settings-radio-t-96-khz-48-khz = 96 kHz（±48 kHz）
settings-radio-t-192-khz-96-khz = 192 kHz（±96 kHz）
settings-radio-t-384-khz-192-khz = 384 kHz（±192 kHz）
settings-radio-i-q-centre-offset = I/Q 中心偏移
settings-radio-how-far-radio-s-i =
    电台的 I/Q 输出在其自身表盘之上居中的距离，适用于接收中频已偏离零点的电台——例如在 Elecraft 上，将 MENU:RX SHFT 设为 8.0 而非 NOR，这会使表盘避开混频器的直流尖峰（并避免附近强大的 SSB/AM 电台被误作 AM 解调）。

    这不是变频器：电台仍然以真实频率显示和发射，此处的任何内容都不会发送给它。它只说明声卡上的采样位于何处，数据流在到达时会被平移回表盘上。

    除非你已开启此类菜单项，否则保持为 0。如果信号出现在偏移距离两倍的位置，则说明符号相反。
settings-radio-iq-correction = IQ 校正
settings-radio-cancel-mirror-image-every-signal =
    消除每个信号的镜像，以及瀑布图中间的直流尖峰。

    电台的 I/Q 输出是两条模拟通路，它们的增益从来不完全相等，相位也不正好相差 90°，由此产生的是每个信号关于调谐频率的一个反射副本，通常低 30–40 dB——强到足以看起来像一个并不存在的电台，并被当作电台解码。带有前面板平衡微调的电台，调整的正是这一点；此处则根据接收到的噪声进行测量，无需设置。

    请保持开启。如果你要收听恰好调谐在载波上的 AM，请关闭它——载波是直流，因此会与尖峰一同被消除——或者通过观察信号是否保持存在，来检查它是否真实。
settings-radio-cancel-mirror-images = 消除镜像
settings-radio-dc-notch = 直流陷波
settings-radio-widen-hole-taken-out-middle =
    加宽从跨度中间挖去的凹口，适用于中心尖峰比其下方偏移更宽的电台。

    0 为普通的阻塞器，宽度约为几十赫兹，只去除偏移而不影响其他内容。调高后，跨度底部也会一起被去除：一阶高通滤波器，在此处设定的数值处下降 3 dB，并在其下方继续下降。

    它以电台 I/Q 的中心为中心，即表盘所在处，除非上面的偏移已将其移开——因此调谐在那里的任何信号也会被去除。600 Hz 的 CW 音调位于 600 Hz 的设置之内。
settings-radio-panadapter-bw = 全景图带宽
settings-radio-cat-family = CAT 系列
settings-radio-rigctld-address = rigctld 地址
settings-radio-host-port-running-hamlib-rigctld =
    正在运行的 Hamlib rigctld 的 主机:端口——127.0.0.1:4532 是本机上它自己的默认值。

    例如可用 `rigctld -m 2028 -r /dev/ttyUSB0 -s 38400` 启动，其中 -m 是你电台的 Hamlib 型号编号（`rigctl -l` 可列出它们）。

    这是通用选项：它只能控制频率、模式、PTT、功率、S 表和驻波比，别无其他。若上面的某个原生系列适用于你的电台，它能做得更多——例如从电台自身的文字缓冲区按键、接收滤波器以及按型号的表头刻度。
settings-radio-flrig-address = flrig 地址
settings-radio-host-port-running-flrig-127 =
    正在运行的 flrig 的 主机:端口——127.0.0.1:12345 是本机上它自己的默认值。flrig 在运行时始终提供 XML-RPC；端口位于其 Config → Setup → Server 中。

    与 Hamlib 选项一样，它驱动的是守护进程而非电台本身，但通过 flrig 自己的按型号驱动程序进行——对于许多电台，其功率和滤波器处理比两者中的另一个更为忠实。它可以控制频率、模式、PTT、发射功率（以整瓦为单位）、接收带宽、S 表、驻波比和输出功率。CW 通过 flrig 自己的 cwio 端口发送，该端口须在 flrig 本身中配置。
settings-radio-serial-port = 串口
settings-radio-select = — 请选择 —
settings-radio-baud = 波特率
settings-radio-data-bits = 数据位
settings-radio-parity = 校验位
settings-radio-stop-bits = 停止位
settings-radio-force-rts = 强制 RTS
settings-radio-force-dtr = 强制 DTR
settings-radio-ptt-method = PTT 方式
settings-radio-how-transmit-keyed = 发射是如何被按下的。
settings-radio-how-transmit-keyed-network-link = 发射是如何被按下的。网络链路没有控制线，因此 DTR 和 RTS 在此都不起作用——请使用 CAT（它要求守护进程按下电台）或 VOX。
settings-radio-mode-control = 模式控制
settings-radio-digimode-mode = 数字模式下的模式
settings-radio-what-put-radio-mode-sdroxide =
    对于 sdroxide 通过其声卡调制的模式，应将电台设为何种模式。“DIGI”选择电台的 DATA/PKT 位置，该位置从 USB 或 ACC 输入获取发射音频，并且不经过话筒通路的语音处理；“USB”则保持普通的边带，适用于已设为从其数据输入调制 SSB 的电台，或完全没有 DATA 位置的电台。

    它涵盖所有数字模式——FT8、FT4、PSK、RTTY、SSTV 等，以及以“声卡”方式发送的 CW——并会覆盖这些模式下的模式控制。SSTV 是其边带随频段变化的模式，因此在这里它指的是该边带上的 DATA 位置：40 米以上为 USB-D，40 米及以下为 LSB-D。SSTV-FM 不在此列——FM 载波没有可供选择的边带。
settings-radio-cw-keying = CW 键控
settings-radio-how-cw-panel-s-keyer =
    CW 面板的键控器如何发射。“电台键控器”将电台设为 CW，并把要发送的文字交给其自己的键控器。它使用电台的键控速度（由面板上的 WPM 设定）；在 Yaesu 上，它通过键控器记忆 1 发送，会覆盖其中原有的内容。

    它需要电台上开启断点插入（break-in），sdroxide 会在 Icom、Yaesu 和 Kenwood 上随每条消息自动开启它——断点插入关闭的电台会接收该消息，但不发射任何内容。之后它会保持开启，完全断点插入永远不会降为半断点。

    “声卡”改为将键控的侧音作为音频发送（MCW），音调为表盘 + 音调偏移——由于 CW 模式下的电台会完全忽略其声卡，因此选择 CW 时会改为遵循数字模式设置（USB、DATA 或“由电台控制”），而不是把电台切换为 CW。对于无法通过 CAT 下达数据位置命令的电台（例如 Xiegu 的 U-D），请在那里选择“由电台控制”，并像 FT8 那样自行将电台停在该位置上。
settings-radio-poll-rate = 轮询速率
settings-radio-how-often-radio-asked-what =
    多久询问一次电台正在做什么——它的表盘、模式和表头。这是控制链路中从电台返回 sdroxide 的那一半：转动电台自己的 VFO 旋钮或在前面板更改模式，读数、频段和全景图会在一次轮询之内跟随。

    这也是此端产生的全部控制通信量，而在现代 Icom 上，这些通信并非免费：CI-V 端口与声卡位于电台内部的同一个 USB 集线器之后，因此此处请求的帧会占用音频本应获得的总线时间——表现为看起来像 DSP 故障、实则不是的丢音。默认的 2 Hz 比旋钮慢半秒，但足够安静，不会干扰音频。如果控制端口是独立设备（单独的 USB 串口适配器、网络 rigctld），则可以提高；如果电台的音频断断续续，则请降低。

    在 Icom 自己的菜单中开启了 CI-V Transceive 的电台，无需为表盘做这些：它会在旋钮转动的瞬间上报，sdroxide 一看到它这样做，就会停止表盘轮询——而如果电台在不作声明的情况下移动，它又会恢复轮询。

    速率由表盘决定。模式只在每四次轮询时才顺带查询一次，因为它是一个在一晚上只变化几次的设置，而不是跟随旋钮变化的数值。
settings-radio-send-command = 发送命令
settings-radio-which-transceiver-generation-keys-rig =
    对于 PTT 方式“CAT”，选择哪一代收发信机来按下电台。两者对 TX 参数的含义各不相同，而且线路上没有任何信息能区分它们，因此请选择与你的电台相符的一项。

    “TS-2000 风格（TX;）”——TS-480、TS-570、TS-870、TS-2000：普通发射，在主频段上。

    “TS-590 风格（TX1;）”——TS-590S/SG、TS-890、TS-990：DATA SEND，它通过 ACC2/USB 音频输入按下发射。在这些电台上，普通发射则改为选择话筒，并将 sdroxide 发射的音频静音。

    若设置错误，TS-590 会发射静音——而 TS-2000 则会在副频段上发射，那是完全不同的另一个频段。
settings-radio-radio = 电台
settings-radio-k3-k3s-kx3-kx2-k4 = K3 · K3S · KX3 · KX2 · K4
settings-radio-one-profile-covers-family-k3 =
    一个配置文件涵盖整个系列：K3 命令集，KX3、KX2 和 K4 都支持它。此处无需选择——驱动滑块的范围（裸机 KX2 或 KX3 为 12 瓦，搭配 KPA3 或 KXPA100 则为 110 瓦）是在端口打开时，从电台自身的选件模块查询中读取的。

    请注意上面的波特率：K3、K3S、KX3 和 KX2 最高只能到 38400，而低于此处所选速率的电台则完全不会应答。
settings-radio-transmit-input = 发射输入
settings-radio-where-radio-takes-transmit-audio =
    电台从何处获取其发射音频——即电台的 TI 命令，它在前面板的菜单 32“TX IN”中。端口打开时设置。

    “USB 音频”是让此接口工作的关键：FDM-DUO 会发射 sdroxide 写入其 USB 声卡的内容。若电台保持在“话筒”，则会改为发射房间里的声音，而屏幕上不会有任何提示。

    “自动”由电台自行选择——对话筒上的 PTT 按下时使用话筒，CAT 或 RTS 按下发射时使用 USB 端口。

    “保持电台上的设置”则完全不发送 TI，适用于在前面板设置此项的操作员。
settings-radio-which-two-sockets-back-receiver =
    接收机监听背面两个插座中的哪一个——即电台的 AN 命令，在前面板的菜单 31“ANTENNAS”中，并在其显示屏上以“ANT 1 2”指示。

    “RTX”是一根天线同时承担两项工作，位于同时承载发射的 M 型插座上。“仅接收”则把接收移至第二个插座，发射仍保留在 RTX 上——即接收天线，例如环形天线或 Beverage 天线，而波束仍接在发射机上。

    立即生效，并在端口打开时从电台读回：这是电台自身的设置，而非此处保存的副本。
settings-radio-one-profile-covers-both-there =
    一个配置文件涵盖两者。此处无需选择——端口打开时电台会自报名称。

    这仅是 CAT 部分：它通过电台的 CAT USB 端口控制表盘、模式、PTT、S 表、驻波比和发射功率，并从其 USB Audio 端口获取音频。FDM-DUO 的第三个 USB 端口传输宽带 I/Q，本接口不读取它——若需要该功能，请在上面选择 ELAD 接口。

    请注意波特率：电台上的菜单 70“CAT BAUD”必须与之匹配，出厂设置为 38400。

    CW 由电台自己的键或电键按下。FDM-DUO 没有接受文字的命令，因此 CW 面板无法通过 CAT 对其进行键控。
settings-radio-one-profile-covers-range-qrp =
    一个配置文件涵盖整个系列：QRP Labs 自己的命令集，它是 Kenwood TS-480 命令集的子集，并增加了很多内容。此处无需选择——端口打开时电台会自报名称及其固件版本，而该版本决定是否要请求读取驻波比保护。

    不要使用 Kenwood 配置文件：在 QMX 上，PC 命令是功率*计量*而非功率*控制*，因此若将电台按 Kenwood 驱动，就会把计量读数当作设置去写入；而 MD8——即 Kenwood 的模式——在这里是驻波比调谐。

    发射功率在电台上设置。没有用于此的 CAT 命令，因此驱动滑块只能调整输入声卡的音频电平。

    接收滤波器也归电台所有：它会报告其模式所隐含的带宽（数字模式为 3.2 kHz，CW 为 300 Hz），但没有提供任何更改它的方式。
settings-radio-baud-rate-ignored-port-usb = 波特率被忽略——该端口为 USB
settings-radio-qmx-serves-own-virtual-com =
    QMX 通过 USB 提供它自己的虚拟 COM 端口，因此上面设置的速率对两端都没有影响。它最多提供三个此类端口；如果电台无应答，最可能的原因是选错了端口。

    关于该端口有一点确实重要：发送给它的回车会使电台在本次会话的剩余时间内切换到终端模式。sdroxide 从不发送回车——但若同一端口上留有一个终端程序，它就会发送，而一旦发送，CAT 便会立即停止工作。
settings-radio-i-q-mode-switched-radio = I/Q 模式在电台上开启
settings-radio-radio-s-sound-card-carries =
    电台的声卡承载的，要么是解调后的音频，要么是其 ADC 所见的原始 I/Q；端口打开时，sdroxide 会按上面“音频格式”的要求设置其中之一（即电台的 Q9 命令，即菜单中的“IQ mode”项）。

    上面的中心偏移和采样率已设为 QMX 所需的数值：其接收机是一台带 12 kHz 中频的超外差机，因此合成器位于表盘以下 12 kHz，声卡上的所有内容都位于跨度中心之上 12 kHz。声卡以 48 kHz 运行，这使全景图宽 48 kHz。

    在 CW 模式下，电台会将该中频再移动约 700 Hz，而单一数值无法跟随这一变化——若你在 CW 模式下使用全景图，请自行加上这一偏移。在 Digi 模式下（作为 I/Q 前端使用的电台通常处于此模式），则无需加任何偏移。

    QRP Labs 指出，I/Q 模式不适用于 WSJT-X 以及其他期望收到解调音频的程序；在这里，解调是在此端完成的，因此可以使用。
settings-radio-rs-hfiq-5-w-hf = RS-HFIQ（5 瓦 HF 收发信机）
settings-radio-hobbypcb-s-rs-hfiq-there =
    HobbyPCB 的 RS-HFIQ。此处无需选择：该配置文件即为电台命令集的全部，端口打开时会记录其固件版本。

    控制链路只传输两样东西，别无其他——振荡器应设在哪里，以及是否发射。由于电台没有模式、功率、滤波器、静噪或计量命令，因此也就没有相应的命令。其余一切都由 sdroxide 完成：从声卡输出的是以表盘为中心的复数基带，因此模式、滤波和调制都在这一端进行。

    频率命令涵盖 3–30 MHz。若请求超出此范围，电台会拒绝并保持原位；日志中会说明此情况。

    CW 通过发射通路以音频方式键控——该电台没有能接受文字的键控器。（它有内部 CW 发生器，但 sdroxide 从不使用它，这是遵从固件本身的建议。）
settings-radio-sound-format-baud-ptt-are = 音频格式、波特率和 PTT 由电台决定，而非设置项
settings-radio-all-three-have-been-filled =
    这三项都已填写，且没有一项是偏好设置：

    • 音频格式为 I/Q（立体声）——声卡只承载复数基带，别无其他。
    • 57600 波特，8N1——这是固件唯一的速率，没有可更改的菜单。任何其他速率都会形成静默的链路，因此在端口打开时即被固定。
    • 通过 CAT 控制 PTT——*X 命令是该接口唯一的发射开关。

    中心偏移已设为零：合成器以表盘频率的四倍工作，送入正交检测器，因此跨度中心即为表盘。请将上面的采样率设为你的声卡实际运行的速率；正是这一点决定了全景图的宽度。
settings-radio-dl2man-pe1nnz-s-pocket-qrp =
    DL2MAN/PE1NNZ 的袖珍 QRP 收发信机，以及由它发展而来的开源 uSDX 固件。它模拟 Kenwood TS-480，但子集很薄：它只应答表盘、模式、PTT 以及 RIT/XIT 清除。没有 S 表、驻波比、功率控制、VFO B、异频和 CAT 键控器，因此驱动滑块和计量在 CAT 上不起任何作用——音频电平是唯一的发射控制。

    CW 通过边带以音频方式键控（MCW），或在电台上用键或电键；没有能接受文字的命令。
settings-radio-audio = 音频
settings-radio-tr-usdx-has-no-sound =
    (tr)uSDX 没有自己的声卡，只有两种方式可以被听到，哪一种正确取决于你的操作环境，而非电台本身。

    “单根线缆”将音频放在 CAT 串行链路本身之中——即电台自身的 8 位数据流，与控制信号通过同一根 USB 线缆传输，无需插接其他任何东西。其缺点在于固件：数据流运行时它无法接受 CAT 命令（发送到数据流中的命令不会使其暂停，而是直接终止它），因此此模式从不轮询。调谐和模式更改会在你操作时发送，每次都会使数据流暂停片刻再重新开始，因此每次更改都会有短暂的间隙；仅仅收听时则不发送任何内容。电台自己的表盘和模式也不会被跟随——没有可供跟随的读取——因此请从这里进行操控。

    “USB 声卡”则从接在电台 3.5 毫米扬声器/话筒插孔上的外置声卡获取音频，并像任何其他 CAT 电台一样工作：表盘轮询照常进行，因此电台自己的旋钮和模式会被跟随。请在下方的“电台音频”中选择声卡。

    两种模式打开端口时都会将 DTR 置为高电平：串口适配器的 DTR 接在电台的复位上，因此打开端口会使其重启，在它启动的最初一两秒内保持安静。DTR 从不用于键控。
settings-radio-radio-model = 电台型号
settings-radio-which-icom-two-things-ci =
    选择哪一款 Icom，因为 CI-V 在其中部分型号上的两件事情的做法不同。

    下面的收发信机地址会根据它自动填入：每个型号的地址各不相同，发送到错误地址的帧会被直接忽略——电台无应答，而且没有任何错误提示说明原因。

    它还决定 sdroxide 能否选择 DATA 模式。在 CI-V 上，USB 和 USB-DATA 使用相同的模式字节，由一条单独的命令加以区分；没有这条命令，数字模式的发射就会经由话筒输入发出，并经过电台的语音处理和 SSB 滤波器。

    “其他”由你自行填写地址，并且完全不发送 DATA 模式命令。
settings-radio-radio-id-hex = 电台 ID（十六进制）
settings-radio-show-radio-s-spectrum-scope = 显示电台的频谱示波器
settings-radio-scope-span = 示波器跨度
settings-radio-how-wide-sweep-radio-keeps = 扫描的宽度。电台会保留其自身屏幕上最后选择的跨度——通常为几 kHz。在此设置跨度还会使示波器进入中心模式，从而跟随表盘。它也会改变电台自身的显示；“保持电台上的设置”则不作改动。
settings-radio-press-apply-reconnect-switch-without = 按“应用 / 重新连接”即可无需重启地切换。
settings-radio-radio-audio-sound-card = 电台音频（声卡）
settings-radio-waiting-sound-cards-machine-radio = 正在等待电台所插入机器上的声卡。
settings-radio-radio-rx = 来自电台（RX）
settings-radio-radio-tx = 发送至电台（TX）
settings-radio-apply-reconnect = 应用 / 重新连接
settings-radio-reopen-cat-rig-these-sound = 使用这些声卡重新打开 CAT 电台——无需重启
settings-radio-reconnects-radio-without-restarting = 无需重启即可重新连接电台。
settings-radio-receive-radio-pc = 接收（电台 → 电脑）
settings-radio-sound-card-radio-s-own = 电台自身音频进入的声卡——即其耳机插孔接入电脑的线路输入或话筒输入。全景图和解码器中的所有内容都从这里进入。
settings-radio-transmit-pc-radio-mic = 发射（电脑 → 电台话筒）
settings-radio-sound-card-carries-audio-radio = 承载电台必须发射的音频的声卡——即电脑的输出接入电台的话筒插孔。一旦音频到达此处，电台便会通过 VOX 自行键控，因此除非正在通过电台发射，否则此设备是静音的。
settings-radio-radio-has-no-control-cable = 此电台没有控制线缆——它通过 VOX 自行键控，因此没有可以下达命令的 PTT。发射即是输入到电台话筒的音频。
settings-radio-reopen-radio-these-sound-cards = 使用这些声卡重新打开电台——无需重启
settings-radio-devices = 设备
settings-radio-discover = 搜索
settings-radio-no-devices-press-discover = 没有设备——请按“搜索”
settings-radio-manual-ip = 手动 IP
settings-radio-optional-e-g-192-168 = 可选，例如 192.168.1.50
settings-radio-sample-rate = 采样率
settings-radio-receiver-ddc = 接收机（DDC）
settings-radio-protocol-2-board-carries-several = Protocol 2 板卡可在一条连接上承载多个可独立调谐的接收机（DDC）——例如在 DDC1 上运行此电台，在 DDC2 上运行另一台电台（地址相同）。发射机属于 DDC1 的电台。Protocol 1 板卡只有 DDC1。
settings-radio-lna-gain = LNA 增益
settings-radio-front-end-gain-hermes-lite = Hermes-Lite 2 的前端增益。立即生效——无需重新连接——并会被记住为电台启动时的电平。过高会使 ADC 削波，整个频段看起来失真；过低则接收机会变聋。
settings-radio-overload-protection = 过载保护
settings-radio-back-lna-gain-off-itself =
    当板卡报告其 ADC 过载时，自动回退 LNA 增益，待过载停止后再将其恢复。

    Hermes-Lite 2 将整个 0–38 MHz 采样到一个 12 位变换器上，其前端既没有混频器也没有预选器，因此相隔一个频段的广播电台就可能使其过载，而你所观察的频段却完全正常——噪声底升高，一切都互调，解码器随之停止工作。板卡会察觉这一点，而这里的设置就是对其作出反应的机制。

    默认关闭：它会移动你设置的控件。发射期间不做任何动作。
settings-radio-wind-gain-back-when-adc = ADC 过载时回调增益
settings-radio-applies-apply-reconnect-main-window = 在“应用 / 重新连接”时生效。主窗口的增益滑轨读取板卡实际运行的增益，因此会跟随此循环，你可以观察它的动作；上面的滑块是电台启动时的电平，循环不会改写它。
settings-radio-step = {"  "}步进
settings-radio-how-far-gain-moves-each = 每次增益移动的距离，单位为分贝。一分贝即为板卡自身增益寄存器的步进。
settings-radio-attack-decay = {"  "}攻击 / 衰减
settings-radio-how-often-gain-may-come =
    当变换器过载时，增益可以多久下调一次；过载停止后，又可以多久上调一次。

    两者有意相差一百倍。下调要立即进行：每一毫秒的过载都意味着接收机充满了互调。上调则要缓慢：无论造成过载的原因是什么——邻近电台发射，或傍晚广播电台开始播出——通常都没有消失，如果循环回升与回退一样快，它就会在整个晚上在阈值两侧振荡。每分贝 100 毫秒和 10 秒，正是 PowerSDR 的 Auto S-Att 和 N1GP 的 HermesIntf 所采用的数值。
settings-radio-range = {"  "}范围
settings-radio-lowest-highest-gain-loop-may = 循环可以使用的最低和最高增益，单位为分贝。上限的作用是防止它替你决定接收机的灵敏度；下限则是你设定的界限：低于某一点时，过载就成了别人的问题，解决办法是加滤波器，而不是再多加二十分贝。
settings-radio-filter-board = 滤波器板
settings-radio-accessory-board-hermes-lite-2 = Hermes-Lite 2 的 J16 接口上的附件板（或任何其他 openHPSDR 板卡的开集电极输出——包括 Protocol 2）。“N2ADR”为每个频段选择一个继电器；“Alex / Hermes 频段代码”将频段编号放在输出 1–4 上，这是 ANAN 的 Alex 板、Zeus SDR、HiQSDR 和 Quisk 所期望的。“自定义”会打开下方的一个表格，由你逐频段、分别为接收和发射指定七个输出——用于天线开关、功放的频段译码器或其他两种预设都不适用的情况；任一预设都可以先复制进去作为起点。除非确实安装了相应设备，请保持为“无”：这七个引脚是通用的开集电极输出，操作员也会把它们接到功放 PTT、天线继电器和变频器切换上。若根据频段数据驱动它们，就会开始操作所连接的任何设备。在“应用 / 重新连接”后生效。
settings-radio-io-board-rx-input = IO 板 RX 输入
settings-radio-where-n2adr-hl2ioboard-takes-receive = N2ADR HL2IOBoard 从何处获取其接收信号。板卡本身会被自动识别，无需设置；此项仅供已将其自身 SMA 插座接线的操作员使用。否则请保持为“电台自身的输入”——若选择 IO 板的 J9 但其上未接任何东西，接收机就会变聋。在“应用 / 重新连接”后生效。设为任一 J9 选项后，这些输入也会出现在 ANT 控件中，并按频段记忆。
settings-radio-power-amplifier = 功率放大器
settings-radio-use-hermes-lite-2-s = 使用 Hermes-Lite 2 的板载功放
settings-radio-default-what-you-want-unless = 默认开启，除非外置功放由板卡的低功率 RF1 输出驱动，否则这正是你需要的设置。关闭时电台仍会键控——收发继电器会动作，任何附件板也会跟随——但天线插孔完全不输出功率，并且继电器被有意保持在接收状态。仅对 Hermes-Lite 板卡有效，其他板卡忽略此项。
settings-radio-mirror-board-s-spectrum-about = 以调谐频率为轴镜像板卡的频谱，发射和接收都适用。默认开启：Hermes-Lite 2 需要它。只有当信号出现在表盘的错误一侧而且无法解码时才应关闭——明显的迹象是：瀑布图上充满了逼真的轨迹，而 SSB 落在了错误的边带上，FT8 则完全没有解码结果。
settings-radio-frequency-correction = 频率校正
settings-radio-crystal-tcxo-error-ppm-applied = 晶体/TCXO 误差，单位为 ppm，同时应用于接收和发射。立即生效。
settings-radio-ppm = {" "}ppm
settings-radio-transmit-buffer = 发射缓冲
settings-radio-how-far-ahead-real-time = 发射音频通过网络被送入板卡时，领先实时多远，之后 sdroxide 才会放慢速度以与之匹配。板卡本身不保存此类缓冲——这仅会扩大 sdroxide 自身的余量。在 WiFi 链路或 VPN 上应提高它：对于直连有线连接而言正确的较低默认值，不足以抵御抖动，发射音频或 PTT 会出现卡顿；数值越高，发射延迟越大。在“应用”后生效，此操作会重新连接板卡。
settings-radio-puresignal = PureSignal
settings-radio-adaptive-predistortion-linearise-transmitter-sample = 自适应预失真：根据发射机实际输出的一份样本使其线性化，在相同功率下可减少二十余分贝的互调。接收机是反馈通路——板卡在通话期间持续接收——因此定向耦合器和衰减器必须把功放输出的一份样本送入一个发射时不会被收发开关切断的输入。在 Hermes-Lite 2 上，这意味着使用 IO 板的 PureSignal 插孔，并将上面的接收输入设为与之相符；在没有该插孔的板卡上，则使用 ANAN 的 RX 端口，或在通话期间切入耦合器的外部继电器。被读取的是这台接收机——带有第二个 ADC 的板卡无法将 RX2 用作反馈抽头，无论其自身固件如何路由它。若没有耦合任何信号，循环将永远无法锁定，发射机则保持在原本的状态。在“应用 / 重新连接”后生效，并且仅对拥有发射机的电台（DDC1）有效。
settings-radio-correct-transmitter-receiver-s-own = 利用接收机自身的反馈校正发射机
settings-radio-table-steps = {"  "}表格步数
settings-radio-how-finely-amplifier-s-curve = 功放的曲线建模的精细程度。步数越多，越能贴合陡峭的拐点，但填充所需时间也越长；32 是一个合理的起点。
settings-radio-adaptation = {"  "}自适应
settings-radio-how-fast-correction-follows-what = 校正跟随耦合器报告内容的速度。慢速是正确的选择——这是从一条带有噪声的反馈通路中，对一条不会移动的功放曲线做平均。
settings-radio-hold = 保持
settings-radio-stop-adapting-keep-correction-where = 停止自适应，并将校正保持在当前位置——用于测量，以及对其已学习的结果感到满意的操作员。
settings-radio-manual-ip-overrides-discovery-press = 手动 IP 会覆盖搜索结果。按“应用 / 重新连接”即可无需重启地切换。
settings-radio-open-collector-outputs-band = 按频段的开集电极输出
settings-radio-one-control-word-per-band = 每个频段一个控制字，与你的硬件文档所述一致：位 0 为输出 1，位 6 为输出 7。在该频段接收时 RX 有效，发射机按下时 TX 有效——对于滤波器，两者设为相同的值；对于只应属于切换某一侧的设备（功放的按键线路、接收前置放大器的旁路），则设为不同的值。保持为 00 的频段不输出任何信号。在“应用 / 重新连接”后生效。
settings-radio-alex-band-code = ALEX 频段代码
settings-radio-replace-table-what-preset-would = 用此预设在每个频段上会发送的内容替换该表格，作为可供编辑的起点。配置中的其他内容不会改变。
settings-radio-clear = 清除
settings-radio-every-output-off-every-band = 每个频段上的所有输出都关闭，这与“无”的效果相同。
settings-radio-band = 频段
settings-radio-rx = RX
settings-radio-tx = TX
settings-radio-outputs-asserted = 已输出的信号
settings-radio-other = 其他
settings-radio-everywhere-outside-amateur-bands-short = 业余频段之外的所有位置——短波收听，以及变频器表盘落到任何频段都未覆盖之处。
settings-radio-dongle = 加密狗
settings-radio-rescan = 重新扫描
settings-radio-re-list-usb-bus-no = 重新列出 USB 总线。不会打开任何设备，因此在接收时按下也是安全的。
settings-radio-no-dongles-press-rescan = 没有加密狗——请按“重新扫描”
settings-radio-first-one-found = — 找到的第一个 —
settings-radio-rtl2832u-s-resampler-reaches-225 = RTL2832U 的重采样器覆盖 225–300 kHz 和 900 kHz–3.2 MHz，两者之间没有其他范围。在“应用”后生效。
settings-radio-often-drops-samples = {"  "}（常常丢失采样）
settings-radio-agc = AGC
settings-radio-manual-setting-measurement-weak-signal = 手动是用于测量和弱信号数字模式的设置。调谐器与解调器各有独立的自动环路。
settings-radio-tuner-gain = 调谐器增益
settings-radio-applies-immediately-no-reconnect-tuner = 立即生效——无需重新连接。调谐器有 29 个离散步进，因此数值会贴合到其能够产生的最接近的一档。调谐器 AGC 运行时忽略此项。
settings-radio-crystal-error-parts-per-million = 晶体误差，单位为百万分之一。以 RUST_LOG=sdroxide_rtlsdr=debug 运行，日志会在约 20 秒后打印测得的时钟误差——这就是应当输入的数值。立即生效。
settings-radio-hf-reception = HF 接收
settings-radio-tuner-itself-starts-24-mhz =
    调谐器本身从 24 MHz 开始。RTL-SDR Blog V4 在硬件上对其以下的频率做上变频；其他加密狗只能通过直接对 ADC 采样才能接收 HF，即经由 V3 的 HF 端口。“自动”会把 24 MHz 以下的全部交给 ADC，而把以上的交给调谐器。

    直接采样覆盖每一个 HF 频段，包括 17 米和 15 米——它们在 ADC 的第二奈奎斯特区中到达，且方向正确。然而，没有任何东西滤除 ADC 的输入，因此 28.8 MHz 减去表盘频率处的信号也会一同进入：17 米下方 10.7 MHz，15 米下方 7.726 MHz。

    切换模式会短暂中断数据流。
settings-radio-removes-dongle-s-own-dc =
    通过测量采样本身的不平衡，从跨度中心去除加密狗自身的直流尖峰，以及每个信号围绕它留下的镜像——无需校准，并且立即生效。调谐器没有偏移调谐模式，因此这是清除中心的唯一方法。

    恰好调谐在表盘上的 AM 载波也位于直流处，因此会与尖峰一同被去除：请将调谐点偏离它一千赫兹，或关闭此项。
settings-radio-remove-centre-spike-mirror-image = 去除中心尖峰和镜像
settings-radio-bias-tee = 偏置三通
settings-radio-feed-4-5-v-dc = 通过同轴电缆供给约 4.5 V 直流
settings-radio-bias-tee-never-connect-transceiver = 偏置三通已开启。此功能启用期间，切勿连接收发信机、接地天线或由别处供电的前置放大器——直流会直接沿馈线进入。
settings-radio-receive-only-dongle-sample-rate = 仅接收。加密狗和采样率在“应用”后生效；其他设置在你更改时即生效。
settings-radio-server-address = 服务器地址
settings-radio-where-rtl-tcp-listening-address =
    rtl_tcp 正在监听的位置：一个地址，或一个地址加端口。端口默认为 1234，即 rtl_tcp 自身的默认值。

    在远端，请以 `rtl_tcp -a 0.0.0.0` 启动它——绑定到 127.0.0.1（不带 -a 时它的行为）则只接受来自同一台机器的连接。

    在“应用”后生效。
settings-radio-host-host-port-e-g = 主机或 主机:端口，例如 raspberrypi.local:1234
settings-radio-requested-server-same-resampler-limits =
    向服务器请求，其重采样器限制与本地加密狗相同——远端使用的是同一块芯片。

    每个采样率旁的数字表示它在链路上的开销：采样数据未经压缩发送，而网络无法承载的采样率会使 rtl_tcp 断开连接，而不是降级。在“应用”后生效。
settings-radio-wired-link = {"  "}（有线链路）
settings-radio-runs-server-s-dongle-manual = 在服务器的加密狗上运行。手动是用于测量和弱信号数字模式的设置。
settings-radio-applies-immediately-no-reconnect-sent = 立即生效——无需重新连接。以十分之一分贝为单位发送，并由服务器贴合到其调谐器所具有的步进；该协议没有回复，因此无法读回它最终设定的数值，此滑块将继续显示所请求的数值。调谐器 AGC 运行时忽略此项。
settings-radio-crystal-error-server-s-dongle =
    服务器加密狗的晶体误差，单位为百万分之一——它是该硬件的属性，因此在此设置，而非在本机的加密狗上设置。立即生效。

    USB 接口打印的测得时钟误差在这里无法获得：通过网络时，该测量所反映的是缓冲，而不是晶体，并且会相差数千 ppm。请在 USB 上校准一次加密狗并将数值带过来，或者调谐一个频率已知的广播电台，并调整直到它落在表盘上。
settings-radio-tuner-starts-24-mhz-below =
    调谐器从 24 MHz 开始；低于此频率时，远端需要帮助。Blog V4 会自行上变频，而“自动”会让它保持原样——这是它唯一能做的事，因为协议只报告调谐器芯片，而没有其他信息，而且 V4 在线路上与普通的 R828D 无法区分。

    对于 V3 或任何其他加密狗，“自动”会在调谐器自身 24 MHz 的下限以下把服务器切换为直接采样——这覆盖了每一个 HF 频段，包括 17 米和 15 米，它们在 ADC 的第二奈奎斯特区中到达，并且 28.8 MHz 减去表盘频率的镜像叠加其上。对于在 HF 上什么都听不到的普通 R828D，请明确选择“直接采样”。切换会短暂中断数据流。
settings-radio-removes-dongle-s-dc-spike =
    在此处从采样到达时去除加密狗的直流尖峰和镜像——它们是硬件产生的伪影，因此会与其他一切一同通过网络传输。立即生效。

    恰好调谐在表盘上的 AM 载波也位于直流处，因此会与尖峰一同被去除：请将调谐点偏离它一千赫兹，或关闭此项。
settings-radio-powers-preamp-far-end-s = 通过远端的加密狗为前置放大器供电。较旧的服务器没有实现此命令，并会静默地忽略它——协议没有表示“否”的方式——因此偏置三通没有开启，并不一定是这一端的问题。
settings-radio-feed-4-5-v-dc-2 = 通过远程同轴电缆供给约 4.5 V 直流
settings-radio-bias-tee-hardware-somewhere-else = 偏置三通已开启，而其硬件位于别处。该馈线另一端的任何东西——收发信机、接地天线、已经供电的前置放大器——都不在你面前，无法查看。
settings-radio-sdrplay-server-rsp-tcp = SDRplay 服务器（rsp_tcp）
settings-radio-sdrplay-server-greets-exactly-like = SDRplay 服务器的问候方式与加密狗完全相同，因此这些选项始终显示，而不是在检测到设备时才显示。对于普通的 rtl_tcp 服务器，它们会被直接忽略——该协议没有回复，并会丢弃它不认识的命令。服务器一旦自我识别，设备选项卡便会将其标为 rsp_tcp。
settings-radio-input = 输入 A
settings-radio-input-b = 输入 B
settings-radio-hi-z = 高阻
settings-radio-lna-state = LNA 状态
settings-radio-step-index-not-db-figure = 步进索引，而非分贝数：每一步的价值取决于 RSP 型号和频段。0 为最大增益。
settings-radio-if-gain-reduction = 中频增益衰减
settings-radio-reduction-so-more-less-signal = 衰减量，因此数值越大，信号越弱。仅在 RSP 的 AGC 关闭时生效。
settings-radio-enable = 启用
settings-radio-dbfs = {" "}dBfs
settings-radio-level-rsp-s-agc-aims = RSP 的 AGC 力图维持的电平。
settings-radio-notches = 陷波
settings-radio-am = AM
settings-radio-fm = FM
settings-radio-dab = DAB
settings-radio-rf = RF
settings-radio-reference-out = 参考输出
settings-radio-t-24-mhz-clock-out = 24 MHz 时钟输出
settings-radio-rsp2-rspduo-only = 仅 RSP2 和 RSPduo。
settings-radio-receive-only-address-sample-rate = 仅接收。地址和采样率在“应用”后生效；其他设置在你更改时即生效。连接断开后会自动重试，因此服务器重新启动后，无需在此处做任何操作即可恢复。
settings-radio-where-spyserver-listening-address-address =
    spyserver 正在监听的位置：一个地址，或一个地址加端口。端口默认为 5555，即 spyserver 自身的默认值。

    在远端，请检查其配置文件是否绑定了其他机器可以访问的地址——绑定到 127.0.0.1 则只接受来自同一台机器的连接。

    在“应用”后生效。
settings-radio-host-host-port-e-g-2 = 主机或 主机:端口，例如 raspberrypi.local:5555
settings-radio-i-q-bandwidth = I/Q 带宽
settings-radio-how-wide-slice-band-arrives =
    频段中有多宽的一段以 I/Q 形式到达——因此它占用了多少链路。这是*解调*窗口：它跟随表盘，比它更宽的部分则是服务器在全景图上方的条带中的 FFT。

    “自动”以约 96 kHz 为目标，它可以承载此处的所有模式（包括宽带 FM），并且仍能通过蜂窝上行链路传输。在“应用”后生效。
settings-radio-which-stage-server-s-own = 向服务器的自身速率阶梯请求哪一级。每个接收机的阶梯各不相同——它是其最大速率逐级减半——因此这里保存的是级别而非赫兹数，同一设置指向另一台服务器时仍然有意义。

    “自动”以约 1 Msps 为目标。按下“测试连接”，即可查看这些级别在此服务器上对应的数值。在“应用”后生效。
settings-radio-automatic = 自动
settings-radio-sample-format = 采样格式
settings-radio-bits-per-component-wire-so =
    线路上每个分量的位数，以及由此在链路上的开销：16 位是 8 位的两倍，而 32 位浮点则是它的四倍，但所携带的信息并不比接收机的 ADC 更多。

    8 位正是让远程接收机能够通过家用上行链路工作的关键，对几乎所有情况都是正确的。服务器可能会覆盖此设置——有些服务器被配置为坚持某一种格式——并会在日志中说明。在“应用”后生效。
settings-radio-gain = 增益
settings-radio-server-s-gain-stage-index =
    服务器的增益级，以索引表示——而非分贝数。每个索引的含义取决于远端的接收机，并随频段而变化，而协议从不说明，因此此处无法在不凭空捏造数值的情况下将其换算为分贝。

    实际范围由服务器决定，只有连接之后才能知道；超出范围的索引会被钳位。立即生效——无需重新连接。
settings-radio-index = 索引{" "}
settings-radio-digital-gain = 数字增益
settings-radio-how-far-server-scales-samples =
    服务器在将采样量化以用于线路传输之前，向上缩放多少。“自动”会监视实际到达的信号，并将峰值保持在满量程的一半，这是唯一能同时适用于空旷频段和拥挤频段的做法——二者相差三十分贝，没有任何固定数值能同时适用于两者。

    这仅在 8 位时重要：此时远低于满量程的信号会因量化而丢失低位，而高于满量程的信号则会被削平。更宽的格式有充足的余量，并会得到其他客户端所使用的数值，它由增益索引和抽取级别计算得出。立即生效。
settings-radio-full-band-strip = 全频段条带
settings-radio-server-s-own-fft-whole =
    服务器对整个频段进行的 FFT，绘制在全景图上方的条带中。在本界面中，它是唯一的频段视图——全景图本身只与所接收的 I/Q 一样宽。

    关闭它，则接收机将无法看到任何其尚未调谐的内容。立即生效，服务器更改其发送内容时，音频会有短暂中断。
settings-radio-ask-server-low-rate-fft =
    同时向服务器请求整个频段的低速率 FFT 和 I/Q，并将其绘制在全景图上方的条带中。

    它几乎不占用任何开销——每帧几 KB，每秒十几帧——并且它显示的是整台接收机，而不仅是正在解调的那一段。仅在每个字节都很宝贵的链路上才值得关闭。立即生效，音频会有短暂中断。
settings-radio-show = 显示
settings-radio-whole-band = 整个频段
settings-radio-how-much-receiver-strip-covers = 条带覆盖接收机的多大范围。整个频段是可用的最宽视图；缩窄它会把相同数量的频点分布在更少的频谱上，即在较小的范围内获得更精细的细节。
settings-radio-strip-db-window = 条带 dB 窗口
settings-radio-server-quantises-fft-into-window =
    服务器在发送之前，把其 FFT 量化到此窗口内，每个频点一个字节——因此这些设置决定的是条带的测量精细程度，而不仅仅是它的绘制方式。条带*显示*所用的底噪和顶端，由引擎自己的自动电平调整决定，是另一回事。

    默认的 150 dB 是整个协议的范围，无需调整。围绕底噪将其缩窄，可在所有信号都集中在刻度一小段内的接收机上获得更高分辨率。立即生效。
settings-radio-top = 顶部{" "}
settings-radio-range-2 = 范围{" "}
settings-radio-i-q-correction = I/Q 校正
settings-radio-remove-dc-spike-mirror-image = 在本端通过 DSP 去除直流尖峰和镜像。远端的接收机是否需要它取决于其类型——Airspy HF+ 不需要，RTL-SDR 则需要——而协议并不说明它在与哪一种接收机通信。立即生效。
settings-radio-enabled = 启用
settings-radio-test-connection = 测试连接
settings-radio-connect-read-what-server-says = 连接，读取服务器对其接收机的说明，然后再断开——不启动数据流，因此对别人正在使用的服务器按下也是安全的。
settings-radio-receive-only-panadapter-narrow-i = 仅接收。全景图是跟随表盘的窄幅 I/Q 窗口；频段视图则是其上方条带中的服务器 FFT。按下“应用 / 重新连接”即可无需重启地切换。
settings-radio-receive-only-wideband-i-q = 仅接收。宽带 I/Q，与任何本地 SDR 相同。按下“应用 / 重新连接”即可无需重启地切换。
settings-radio-receiver-address = 接收机地址
settings-radio-where-receiver-serves-web-page =
    接收机提供其网页的位置：一个地址，或一个地址加端口。端口默认为 8073，即 KiwiSDR 自身的默认端口。

    请注意，通过本项目代理（something.proxy.kiwisdr.com——约占公共接收机的近一半）连接的接收机，改为在 80 端口上应答，因此请明确填写端口。从“公共 SDR”中选择接收机会正确地自动填入。

    在“应用”后生效。
settings-radio-host-host-port-e-g-3 = 主机或 主机:端口，例如 kiwi.example.org:8073
settings-radio-password = 密码
settings-radio-receiver-s-user-password-where =
    接收机的*用户*密码，在其操作员设置了密码的情况下。几乎所有公共接收机都留空。不是管理员密码。

    在“应用”后生效。
settings-radio-announce = 显示名称
settings-radio-name-connection-shows-up-under =
    此连接显示的名称，接收机的所有者以及收听它的其他人都可以看到。

    留空则使用你的电台呼号，这是该网络的惯例——某些操作员确实会屏蔽不表明身份的客户端。在“应用”后生效。
settings-radio-blank-station-callsign = 留空 = 电台呼号
settings-radio-band-view = 频段视图
settings-radio-ask-receiver-own-waterfall-well =
    同时向接收机请求其自身的瀑布图和 I/Q，并将其绘制在全景图上方的条带中。

    值得开启：I/Q 只有约 12 kHz 宽，因此没有它，就没有可以*据以*调谐的依据。这是第二条连接，全速时约为 20 kB/s，而 I/Q 为 44 kB/s。

    在“应用”后生效。
settings-radio-show-receiver-s-0-30 = 显示接收机的 0–30 MHz 瀑布图
settings-radio-band-view-speed = 频段视图速率
settings-radio-how-often-receiver-sends-waterfall =
    接收机发送瀑布图行的频率，从 1（最慢）到 4。这是此处唯一会改变运行时链路开销的设置，因此在按流量计费的连接上请调低它。

    立即生效。
settings-radio-band-view-span = 频段视图跨度
settings-radio-how-much-band-receiver-s =
    接收机的瀑布图覆盖多大频段。滑块最左端为完整的 0–30 MHz，向右每一步减半。

    无论窗口多宽，瀑布图始终为 1024 个频点宽，因此这实际上是分辨率控制：整个频段每个频点约 29 kHz，它是频段*地图*，而非任何东西的画面；而在广播频段上，几百 kHz 的跨度则能显示单个电台。放大时，窗口会跟随你的表盘。

    整个频段是默认值，也是让条带成为可用于调谐的依据的原因；其下方的全景图则是约 12 kHz 的 I/Q。立即生效。
settings-radio-receiver-agc = 接收机 AGC
settings-radio-receiver-s-agc-which-far =
    接收机的 AGC，它位于链路另一端，且在 I/Q 之前——因此它先于 sdroxide 所做的任何处理而动作。

    与此处的每一台本地 SDR 不同，它默认开启，因为经过测量，另一种选择更差：在一台实际运行的接收机上，手动增益在其范围内并非单调变化，并且其高端会使 I/Q 在满量程处削波，而 AGC 则将电平保持在约 -24 dBFS，并留有 7 dB 余量。

    这确实意味着采样幅度不是信号电平，因此 S 表改为读取接收机自身提供的数值。立即生效。
settings-radio-let-receiver-ride-own-gain = 让接收机自行控制增益
settings-radio-manual-gain = 手动增益
settings-radio-fixed-gain-when-receiver-s = 接收机 AGC 关闭时的固定增益，使用其自身的 0–90 刻度。这不是任何已说明的分贝数——协议称之为 manGain，并不多作说明，而且经测量发现它并非单调变化，因此应将其视为用来寻找良好位置的旋钮，而非校准值。立即生效。
settings-radio-read-receiver-s-status-page = 读取接收机的状态页面：它是什么、覆盖哪些范围、还有多少空闲通道，以及其操作员是否允许浏览器以外的应用连接。不占用它的任何通道。
settings-radio-host-port-e-g-127 = 主机:端口，例如 127.0.0.1:50001
settings-radio-iq-sample-rate = IQ 采样率
settings-radio-receiver = 接收机
settings-radio-rig-two-receivers-sunsdr2dx-can = 带两台接收机的电台（SunSDR2DX）可通过一条连接为两个电台选项卡提供服务——例如在 RX1 上运行此电台，在 RX2 上运行另一台。发射机属于 RX1 的电台。
settings-radio-stream-delay = 数据流延迟
settings-radio-how-long-rig-s-iq = 在 sdroxide 将电台的 IQ 移至新的中心频率后，它需要多长时间才能到达。这是电台自身的 DSP 流水线，而非网络——本机上的服务器同样有这一流水线——而当全景图被完全缩小拖动时，偏差达到这一量级，会使最新的瀑布图行相对于历史记录横向错位，错位量等于误差乘以拖动速度。设得过高或过低，错位的距离相同，方向相反。可用 `cargo run --release -p sdroxide-tci --example retune_latency` 测量它；默认值针对的是在 ExpertSDR3 上以 192 kHz 运行的 SunSDR2DX。
settings-radio-wideband-iq-receive-audio-transmit = 宽带 IQ 接收，音频发射。按下“应用 / 重新连接”即可无需重启地切换。
settings-radio-radio-address = 电台地址
settings-radio-address-shown-radio-under-set = 电台在 SET > Network 下显示的地址。那里必须开启“Network Control”，并且电台需要设置网络用户名和密码。
settings-radio-host-ip-e-g-192 = 主机或 IP，例如 192.168.1.50
settings-radio-control-port = 控制端口
settings-radio-t-50001-unless-has-been-changed = 除非在电台上被更改，否则为 50001。
settings-radio-network-user = 网络用户
settings-radio-stored-clear-radio-json-protocol = 以明文保存在 radio.json 中。协议在线路上以可逆方式对其进行混淆，因此在此处做任何处理都不会使它成为秘密。
settings-radio-receive = 接收来源
settings-radio-af-radio-demodulates-sdroxide-shows = AF：电台解调，sdroxide 显示音频频段。12 kHz IF：电台改为发送其 DRM 中频，由 sdroxide 解调，这使其自身的滤波器、降噪和解码器能够作用于约 ±12 kHz 的范围。无论哪种方式，宽瀑布图都是电台自身的示波器——Icom 不输出 I/Q。“IF”偏好会在 WFM 等较宽的模式下暂时使用 AF，然后在兼容的模式下恢复 IF。“AF”偏好则保持为 AF。此连接上不提供 HD Radio。
settings-radio-audio-sample-rate = 音频采样率
settings-radio-t-12-khz-if-needs-48000 = 12 kHz IF 需要 48000 Hz 的数据流——在此速率下，使用的是电台解调后的音频。
settings-radio-if-spectrum = IF 频谱
settings-radio-which-way-round-radio-s = 电台的 12 kHz IF 以哪个方向运行。“自动”会采用已知该型号的做法——在 IC-7760 上为镜像，在其他所有 Icom 上为正常。若 SSB 出现在错误的边带上，请手动设置：其明显迹象是，在频段运行 LSB 的地方你必须选择 USB，同时电台自己的模式显示始终与 sdroxide 一致。
settings-radio-normal = 正常
settings-radio-mirrored = 镜像
settings-radio-displayed-bandwidth = 显示带宽
settings-radio-width-audio-band-panadapter-cat = 音频频段全景图的宽度，与 CAT 电台相同。用于数字模式以及电台示波器关闭时；否则全景图本身就是示波器，其宽度由“示波器跨度”设定。
settings-radio-how-cw-panel-s-keyer-2 = CW 面板的键控器如何发射。“电台键控器”将电台设为 CW，并把要发送的文字交给其自身的键控器（CI-V 0x17）。“声卡”则改为通过局域网以音频方式发送键控的侧音（MCW），音调为表盘 + 音调偏移——由于 CW 模式下的电台会忽略其调制器输入，因此选择 CW 后会让电台保持在普通的 USB 模式，即数字模式所使用的模式。
settings-radio-how-much-audio-radio-holds = 电台在调制之前保留多少音频。网络越差，保留得越多，代价是发射延迟越大。
settings-radio-streams-radio-s-own-475 = 传输电台自身的 475 频点扫描。在 AF 下，它就是全景图——电台发送的音频是经过其滤波器的结果，而非频段的画面；在 12 kHz IF 下，它则是其上方的全频段瀑布图。无论哪种方式，它都是电台的画面，而非 sdroxide 的 DSP 生成的：没有 I/Q 可供计算它。
settings-radio-how-wide-sweep-only-wide = 扫描的宽度。这是 Icom 唯一的宽视图，而电台会保留其自身屏幕上最后选择的跨度——通常为几 kHz，这就是条带有时无法比其下方的全景图更宽的原因。在此设置跨度还会使示波器进入中心模式，从而跟随表盘。它也会改变电台自身的显示；“保持电台上的设置”则不作改动。
settings-radio-switch-modulation-input-lan = 将调制输入切换为 LAN
settings-radio-transmit-audio-only-heard-when = 只有当电台的 MOD 输入设为 LAN 时，发射音频才会被听到。若 sdroxide 认识该型号的菜单编号，它可以替你写入该设置；对于其他型号，它会说明情况并保持菜单不变。这是临时借用：会话开始时先读取电台在此处保存的内容，会话结束时再将其放回，因此之后单独使用的电台仍会听到它自己的话筒。
settings-radio-copy-diagnostic-report = 复制诊断报告
settings-radio-copies-radio-s-last-session = 将此电台的上一次会话——其握手和 CI-V 跟踪记录——复制到剪贴板，用于错误报告。这是此电台的，而非电台站点的：若局域网上有两台 Icom，每个选项卡都会回应其自己地址的情况。
settings-radio-control-audio-radio-s-own = 控制、音频以及电台自身的示波器，都通过一条网络连接完成——无需串口线缆，也无需声卡。按下“应用 / 重新连接”即可无需重启地切换。
settings-radio-testing = 测试中……
settings-radio-was-only-check-press-apply = 那只是一次检查——请按下方的“应用 / 重新连接”，开始使用此电台。
settings-radio-radios = 电台
settings-radio-asks-network-iio-devices-also = 向网络查询 IIO 设备，并直接尝试 192.168.2.1——通过 USB 线缆连接的 Pluto，即使该地址可以访问，也常常无法通过组播发现。
settings-radio-no-radios-press-discover = 没有电台——请按“搜索”
settings-radio-address = 地址
settings-radio-overrides-selection-above-usb-cable = 覆盖上面的选择。USB 线缆将 Pluto 呈现为网络适配器，而非串口，因此即使电台就在你的桌上，这里填写的也是 IP 地址。
settings-radio-pluto-revision-c-pluto-unlocked = Pluto+ 或已解锁为 2R2T 的 C 版 Pluto，可以从一台设备为两个电台选项卡提供服务——此电台使用 RX1，另一台使用 RX2，各自接在自己的天线上。两条通道共用同一个振荡器，因此重新调谐任一电台都会同时移动两者；RX2 带来的是同一频谱上的第二根天线（分集），而不是第二个频段。发射机属于 RX1 的电台。标准的 1R1T Pluto 在连接时会拒绝 RX2。
settings-radio-width-spectrum-sdroxide-receives-ad9361 = sdroxide 所接收频谱的宽度。AD9361 可达 61.44 Msps；USB 网络链路则达不到，此列表即按此进行缩放。最低的速率需要加载到 AD9361 中的滤波器配置，而 sdroxide 不做这件事——标准的 Pluto 会改为以约 2.084 Msps 运行，并在连接时说明这一点。在“应用”后生效。
settings-radio-more-than-usb-2-will = {"  "}（超过 USB 2 所能承载的范围）
settings-radio-stock-pluto-runs-2-084 = {"  "}（标准 Pluto 运行于 2.084）
settings-radio-analog-filter = 模拟滤波器
settings-radio-ad9361-s-baseband-filter-leave = AD9361 的基带滤波器。保持为 0 即为自动，此时它会打开到采样率的九成——之所以刻意设得很宽，是因为接收机会把其振荡器偏离表盘四分之一个跨度，以使信号远离直流尖峰；窄滤波器会恰好切掉它被移到的那一部分。在“应用”后生效。
settings-radio-ad9361-has-four-modes-not = AD9361 有四种模式，而不是开/关开关。慢速起振适合 SSB 和 CW；快速起振适合突发信号；手动则是用于测量和弱信号数字模式的设置。立即生效。
settings-radio-rx-gain = RX 增益
settings-radio-applies-immediately-no-reconnect-ignored = 立即生效——无需重新连接。除非 AGC 设为手动，否则忽略此项，这是 AD9361 自身的行为，而非 sdroxide 的。
settings-radio-tx-gain = TX 增益
settings-radio-negative-because-ad9361-expresses-transmit = 为负值，因为 AD9361 以衰减表示发射电平：0 dB 即为满功率输出。立即生效。在连接时应用此数值之前，发射机会先被设为最小，因此上一个程序留下的任何东西都不会处于激活状态。
settings-radio-reference-error-parts-per-million = 参考误差，单位为百万分之一。以 RUST_LOG=sdroxide_pluto=debug 运行，日志会在约 20 秒后打印测得的时钟误差——这就是应当输入的数值。由 sdroxide 应用，而不写入电台自身的持久微调。立即生效。
settings-radio-full-duplex = 全双工
settings-radio-keep-receiving-while-transmitting = 发射时继续接收
settings-radio-leave-off-pluto-reached-over =
    通过 USB 线缆连接的 Pluto 请保持关闭。该链路无法同时以每秒一兆采样的速率双向传输，而饿死发射缓冲的通话会被切断发出——因此默认情况下，在通话期间接收会停止。

    对于在真正以太网上的板卡（LibreSDR、千兆适配器上的 Pluto），两者都有余量，可以开启：这样你就能在自己发射的同时听到接收机，QO-100 电台正是以此收听其自己的下行信号。

    通话需要的链路带宽是采样率的两倍——2.5 Msps 即每个方向 10 MB/s——因此如果日志开始报告链路无法承载，请降低采样率。通话期间全景图仍会显示所发射的信号；持续传输的是音频。

    在“应用”后生效。
settings-radio-duplex = 双工
settings-radio-whether-ad9361-runs-both-directions =
    AD9361 是同时双向运行（FDD，即 Pluto 启动时的方式，也是 sdroxide 一直保持的状态），还是一次只运行一个方向（TDD）。

    除非你需要下面的 PTT 引脚，否则请保持 FDD——PTT 引脚正是由 TDD 驱动的，而且它与上面的“全双工”不能兼容。在“应用”后生效。
settings-radio-ptt-pins = PTT 引脚
settings-radio-pluto-s-four-gpo-test =
    Pluto 的四个 GPO 测试点本身即可驱动外部功率放大器、LNA 或收发开关。选择一对引脚后，其中一个在接收期间始终为高电平，另一个在发射期间始终为高电平——无需主机软件参与，也无需接线的串口 PTT 线。

    无论“双工”一行如何设置，这都会使电台进入 TDD，因为这些引脚跟随 AD9361 的使能线，而在 FDD 下这两条线在整个会话中都处于有效状态。“全双工”也会随之关闭，因此如果你从事卫星通信并需要听到自己的下行信号，请两者都保持默认值。

    Analog Devices 自己的说明将外部 LNA 接在 GPO0/GPO1 上，因此如果你的板卡是这样接线的，请使用 GPO2/GPO3。这些引脚约为 1.3 V、只有几毫安：请用它们驱动晶体管或光隔离器，切勿直接驱动继电器线圈。

    在“应用”后生效。
settings-radio-buffer-size = 缓冲大小
settings-radio-how-much-device-holds-before =
    设备在每次传输前保留多少复数采样。默认的 32768 在 2 Msps 下约为 16 毫秒：足够长，使每次传输的往返时间不会成为瓶颈；又足够短，使重新调谐不会明显迟滞。

    如果日志报告接收套接字被替换，请将其减半。链路不稳定——通过 USB 线缆在高采样率下连接的 Pluto 是常见情况——会在传输中途停顿，而较小的传输既不易在飞行中被打断，事后也能更快地补齐。若想以重新调谐延迟换取更少的往返次数，则可调高它。

    在“应用”后生效。
settings-radio-samples = {" "}采样
settings-radio-rx-tx-port = RX / TX 端口
settings-radio-ad9361-s-rf-port-select = AD9361 的 rf_port_select。标准的 Pluto 每个方向各接好一个，因此除非你使用的板卡没有这样接线，否则请留空。在“应用”后生效。
settings-radio-opens-radio-reads-what-says = 打开电台，读取它对自身的说明，并报告此特定板卡的调谐范围。不启动数据流。
settings-radio-copies-last-session-s-protocol = 将上一次会话的协议跟踪记录复制到剪贴板，用于错误报告。
settings-radio-wideband-iq-receive-transmit-half = 宽带 IQ 接收与发射。半双工——在通话期间接收会停止，因为 USB 网络链路无法同时承载两者。无需 SoapySDR，也无需 libiio。
settings-radio-not-yet-verified-against-real = 尚未在实际硬件上验证。若出现异常，请发送诊断报告——它包含与电台交换的每一条命令，以及采样数据流的前几个字节。
settings-radio-flexradio-announces-itself-local-network = FlexRadio 大约每秒在本地网络上宣告一次自身。通过路由器或 VPN 连接的电台永远不会向你广播——请改为在下方输入其地址。
settings-radio-another-gui-client-has-radio = 另一个图形界面客户端正在使用此电台，并且已禁用 multiFLEX。请断开该客户端的连接，或在电台上启用 multiFLEX。
settings-radio-overrides-selection-above-use-radio = 覆盖上面的选择。适用于位于另一子网、VPN 之后或使用非标准端口的电台。
settings-radio-width-spectrum-sdroxide-receives-192 = sdroxide 所接收频谱的宽度。192 kHz 是电台 DAX IQ 数据流的最大值，因此也是此界面能够显示的最宽跨度。
settings-radio-t-192-khz-widest-dax-iq = 192 kHz 是 FLEX 能够发送的最宽 DAX IQ 数据流，而电台会将全景图的跨度与之绑定——因此这就是整个跨度，此处没有任何设置能使它更宽。
settings-radio-dax-iq-channel = DAX IQ 通道
settings-radio-radio-has-four-change-only = 电台有四个通道。只有当网络上的其他设备已在使用通道 1 时才需更改——电台会两次拒绝同一通道。
settings-radio-station-name = 电台站点名称
settings-radio-shown-against-session-radio-s = 在电台的客户端列表中，与此会话对应显示。电台也以此名称记住客户端，因此重命名会使电台将 sdroxide 视为一个新的客户端。
settings-radio-gui-client-id = 图形界面客户端 ID
settings-radio-identity-radio-restores-session-s = 电台用来恢复此会话的切片和全景图的身份。留空时，它由电台站点名称派生——稳定，但在每个保留默认名称的 sdroxide 上都相同，而电台会通过踢掉较早的客户端来处理重复项。sdroxide 能在线路上察觉这一点，并回退到单次会话的身份，这会失去恢复功能；若要保留它，请在此处填入你自己的 UUID。
settings-radio-optional-e-g-uuid-your = 可选，例如你自己的 UUID
settings-radio-mirror-radio-s-i-q =
    以全景图的中心为轴镜像电台的 I/Q。默认关闭，FLEX-6600 即是以此方式验证的。

    若接收音频在 USB *和* LSB 上都无法听懂、且无法解码，而瀑布图看起来完全可信，则可以尝试开启它——这正是镜像数据流的表现，而它是唯一没有其他征兆的故障（问题 #368）。如果不是这个原因，开启此项会使问题变得明显，而不是隐晦。在“应用 / 重新连接”后生效。
settings-radio-network-mtu = 网络 MTU
settings-radio-largest-datagram-radio-may-send = 电台可以发送的最大数据报。1450 是 SmartSDR 自身请求的数值。在 MTU 较小的路径上——VPN 或隧道——请将其调低，否则分片会被丢弃，根本收不到任何频谱。
settings-radio-b = {" "}B
settings-radio-checks-radio-answers-without-registering = 检查电台是否应答，但不注册为图形界面客户端——因此不会干扰已在运行的 SmartSDR 会话。
settings-radio-wideband-iq-receive-over-dax = 通过 DAX 进行宽带 IQ 接收，电台调制的音频发射。按下“应用 / 重新连接”即可无需重启地切换。
settings-radio-not-yet-verified-against-real-2 = 尚未在实际硬件上验证。若出现异常，请发送诊断报告——它包含与电台交换的每一行协议内容。
settings-radio-devices-soapysdr-can-see = SoapySDR 可识别的设备
settings-radio-waiting-machine-radio-attached-where = 正在等待电台所连接的机器，模块安装在那里。它打开哪个设备，取决于该机器 config.toml 中的 `device_args`，或其 --device 参数。
settings-radio-ask-every-installed-soapysdr-module = 要求每个已安装的 SoapySDR 模块进行扫描。不会打开任何设备，因此在接收时也是安全的——但可能需要一点时间。
settings-radio-not-enumerated-yet-press-rescan = 尚未枚举——请按“重新扫描”。
settings-radio-no-soapysdr-devices-found-check = 未找到 SoapySDR 设备。请检查你电台的模块是否已安装，以及你是否有权访问该设备。
settings-radio-not-radio = 不是电台
settings-radio-sound-card-listed-above-if = 上面有一个声卡被列为 SDR。它接受任何频率并忽略它，因此打开它时显示的是声卡的输入，而非频段。sdroxide 不会自动选择它们——但在 device_args 中指名它的那一行仍会被执行。
settings-radio-which-one-opens-set-device = 打开哪一个由 config.toml 中的 --device 或 device_args 设定；两者都没有时，列表中第一台电台生效。
settings-radio-no-device = 无设备
settings-radio-rig-has-no-software-adjustable = 此电台没有可由软件调节的增益。
settings-radio-rx-gains = RX 增益
settings-radio-tx-gains = TX 增益
settings-radio-antennas = 天线
settings-radio-remembered-next-start = 下次启动时记住。
settings-radio-these-controls-appear-once-soapysdr = 这些控件在 SoapySDR 设备打开后才会出现——它们是设备自身的控件，必须向设备查询。
settings-radio-stream = 数据流
settings-radio-follow-app-setting = 跟随应用设置
settings-radio-msps = 或以 Msps 计
settings-radio-baseband-filter = 基带滤波器
settings-radio-let-driver-choose = 由驱动程序选择
settings-radio-changing-either-reopens-radio = 更改其中任何一项都会重新打开电台。
settings-radio-driver-publishes-no-settings-own = 此驱动程序没有公布任何自身设置。
settings-radio-applied-once-remembered-next-start = 立即应用，并在下次启动时记住。
settings-radio-no-rx-888-found-press = 未找到 RX-888——请按“重新扫描”
settings-radio-adc-clock = ADC 时钟
settings-radio-t-129-6-msps-needs-superspeed = 129.6 Msps 需要 SuperSpeed 链路和高速主机；64.8 为安全的默认值。更改它会自动重新打开接收机，这需要一点时间，但无需重启。
settings-radio-panadapter-width = 全景图宽度
settings-radio-hf-only = {"  "}· 仅 HF
settings-radio-usable-both-front-ends = 两种前端均可使用。
settings-radio-below-vhf-crossover-ordinary-width = 低于 VHF 分界点时，这是普通的宽度。高于它时，调谐器的 8 MHz 中频无法居中于这样宽的窗口中，因此实时频谱会偏到全景图的一侧，而其旁边什么都没有。
settings-radio-dial-above-crossover-now-so = 表盘现在位于分界点之上，因此此宽度会将调谐器的 8 MHz 显示为偏离中心，而其旁边是空白的频谱。请缩窄宽度以填满全景图。
settings-radio-vga-gain = VGA 增益
settings-radio-ad8370-variable-gain-amplifier-ahead = ADC 之前的 AD8370 可变增益放大器。
settings-radio-attenuator = 衰减器
settings-radio-pe4304-step-attenuator-0-5 = PE4304 步进衰减器，以 0.5 dB 为步进。
settings-radio-adc-range = ADC 范围
settings-radio-wide-2-25-vp-p = 宽（2.25 Vp-p）
settings-radio-selects-adc-s-wider-input = 选择 ADC 更宽的输入范围：为强广播信号提供更多余量，但弱信号的计数更少。关闭则选择更灵敏的 1.5 Vp-p 范围。
settings-radio-dither = 抖动
settings-radio-adds-small-dither-signal-ahead = 在 ADC 之前加入一个小的抖动信号：以少量底噪为代价，换取无杂散动态范围。
settings-radio-randomiser = 随机化
settings-radio-adc-scrambles-output-so-digital = ADC 会打乱其输出，使数字总线不再向前端辐射；驱动程序会将其还原。除非在调试，否则请保持开启。在重新连接后生效。
settings-radio-dc-hf-antenna-port = HF 天线端口的直流
settings-radio-powers-active-antenna-preamp-down = 通过同轴电缆为有源天线或前置放大器供电。
settings-radio-vhf-tuner-gain = VHF 调谐器增益
settings-radio-r828d-rf-gain-used-above = R828D 射频增益，在自动 HF/VHF 分界点之上使用。29 个离散步进；取最接近的一档。
settings-radio-auto = 自动
settings-radio-let-tuner-run-own-lna = 让调谐器运行其自身的 LNA 和混频器环路。
settings-radio-bias-tee-vhf = 偏置三通（VHF）
settings-radio-dc-vhf-antenna-port = VHF 天线端口的直流
settings-radio-clock-trim = 时钟微调
settings-radio-corrects-reference-oscillator-applied-when = 校正参考振荡器。在你松开数值时应用——若在拖动的每个像素上都重新打开，接收机就会被重启数百次。
settings-radio-receive-only-below-adc-s = 仅接收。在 ADC 的奈奎斯特极限以下，天线被直接采样，重新调谐是即时的，因为没有硬件下变频器——完整的 ADC 数据流由主机转换为基带。在其以上，接收机会自动切换到其 R828D 调谐器和 VHF SMA，因此两根天线都需要接好。VHF 需要 32.4 Msps 或更高的 ADC 时钟，才能使调谐器的中频放得下；在低于 48 Msps 的时钟下，两个范围之间存在无法覆盖的间隙。此处的每项设置都会立即生效——没有需要按下的“应用”按钮。
settings-radio-device = 设备
settings-radio-no-devices-press-rescan = 没有设备——请按“重新扫描”
settings-radio-six-rates-are-six-different =
    这六种速率是六种不同的 FPGA 镜像，而不是一个寄存器——这就是为什么 ELAD 的协议中没有任何东西可以在它们之间进行选择，也是为什么此设置的含义取决于型号。

    FDM-S1 / FDM-S2：是一条命令。它们的 FPGA 在每次上电时都由计算机加载，sdroxide 会运行 ELAD 自己的 elad-firmware 加载器，将此速率的镜像载入。若未安装该加载器，采样器根本不发送任何采样，sdroxide 会在屏幕上说明这一点。

    FDM-DUO：不是命令。电台自行加载其 FPGA，并以其上电时或 FDM-SW2 上次留下的任何速率启动（新机为 192 kHz）；此处的设置仅说明如何*读取*数据流。若设错，全景图的宽度就会不对，其中每个频率都会被相应地缩放。

    数据流的实际速率在开始后几秒钟测得，不匹配会在屏幕上报告。在“应用”后生效。
settings-radio-fdm-s-fpga-image-loaded = FDM-S：加载的 FPGA 镜像——FDM-DUO：数据流的读取方式
settings-radio-receiver-s-input-pad-one = 接收机的输入衰减器——一步，加入或取消。与主窗口的增益滑块为同一控件。
settings-radio-pre-selection-filters = 预选滤波器
settings-radio-low-pass-bank-front-adc = ADC 之前的低通滤波器组。将其旁路可获得最宽的视野，但在强带外信号附近表现最差——几英里外的广播发射机会在频段上产生镜像。除非你有意收听滤波范围之外的内容，否则请保持开启。
settings-radio-filters-circuit = 电路中的滤波器
settings-radio-rig-control-fdm-duo-only = 电台控制——仅 FDM-DUO
settings-radio-transceiver-s-cat-usb-port = 收发信机的 CAT USB 端口，它与接收线缆是分开的线缆。在 FDM-S1 或 FDM-S2 上请保持端口为未设置——它们没有此端口；而在 FDM-DUO 上，你宁可仅通过其接收线缆进行操控：此时仍可调谐和键控，但无法读取任何计量。
settings-radio-port-set-radio-s-display = 设置了端口后，电台的显示与此处的表盘在两个方向上都是相同的数字：FDM-DUO 的接收窗口是围绕其 VFO 构建的，因此在此调谐会移动前面板，而转动前面板旋钮则会移动此处的读数。全景图会随表盘移动而重新居中。若没有端口——或电台不应答的端口——则没有可供命令的 VFO，调谐会停留在电台已在发送的窗口之内。
settings-radio-none-usb-control-only = — 无（仅 USB 控制） —
settings-radio-must-match-menu-70-cat = 必须与电台上的菜单 70“CAT BAUD”一致，出厂设置为 38400。电台只有这四种速率，没有其他，而以任何其他速率打开的端口在两个方向上都是静默的——电台忽略表盘并拒绝键控，且不会说明原因。
settings-radio-how-transmit-keyed-cat-sends = 发射是如何被按下的。“CAT”发送电台自身的 TX 命令，无需在电台上进行任何设置；RTS 则需要将菜单 54“PTT”设为 PTT+RTS。
settings-radio-where-radio-takes-transmit-audio-2 =
    电台从何处获取其发射音频——即电台的 TI 命令，它在前面板的菜单 32“TX IN”中。端口打开时设置。

    “USB 音频”是让此处发射工作的关键：FDM-DUO 会发射 sdroxide 写入其 USB 声卡的内容。若电台保持在“话筒”，则会改为发射房间里的声音，而屏幕上不会有任何提示。
settings-radio-which-two-sockets-back-receiver-2 =
    接收机监听背面两个插座中的哪一个——即电台的 AN 命令，在前面板的菜单 31“ANTENNAS”中，并在其显示屏上以“ANT 1 2”指示。

    “RTX”是一根天线同时承担两项工作，位于同时承载发射的 M 型插座上。“仅接收”则把接收移至第二个插座，发射仍保留在 RTX 上——即接收天线，例如环形天线或 Beverage 天线，而波束仍接在发射机上。

    它会移动整个接收机：全景图、解调器以及电台自身的音频，全都来自此处选择的插座。

    立即生效，并在控制端口打开时从电台读回——这是电台自身的设置，而非此处保存的副本。
settings-radio-how-often-radio-asked-what-2 = 多久询问一次电台正在做什么——它的表盘、模式和表头。转动电台自己的 VFO 旋钮，读数会在一次轮询之内跟随。这也是此端产生的全部控制通信量，因此如果电台的音频断断续续，请调低它。
settings-radio-transmit-audio = 发射音频
settings-radio-system-default = — 系统默认 —
settings-radio-fdm-duo-s-own-usb = FDM-DUO 自身的 USB Audio 端口。若保持为系统默认，它几乎从来不是电台——而是这台机器的扬声器。
settings-radio-press-rescan-list-sound-cards = 按“重新扫描”以列出声卡
settings-radio-copies-last-session-s-trace = 将上一次会话的跟踪记录复制到剪贴板，用于错误报告：与设备交换的每一条命令，以及采样数据流的前几个字节。
settings-radio-only-receiving-fdm-duo-has = 迄今为止，只有 FDM-DUO 上的接收功能在实际硬件上运行过。若出现任何异常，请将诊断报告附在错误报告中。
settings-radio-cw-keyed-radio-s-own = CW 由电台自己的键或电键按下：FDM-DUO 没有接受文字的命令，因此 CW 面板无法对其键控。
settings-radio-press-apply-reopen-radio = 按“应用”以重新打开电台。
settings-radio-no-receivers-press-rescan = 没有接收机——请按“重新扫描”
settings-radio-which-rates-receiver-has-depends = 接收机具有哪些速率取决于型号和固件。在“应用”后生效。
settings-radio-every-rate-any-hf-model = 任何 HF+ 型号提供的每一种速率——连接一台即可查看其自身的速率
settings-radio-receiver-s-own-gain-control = 接收机自身的增益控制。普通收听时请保持开启；若要为测量而手动设置衰减器，请将其关闭。
settings-radio-agc-threshold = AGC 阈值
settings-radio-high = 高
settings-radio-high-trades-little-sensitivity-headroom = 高阈值会以少量灵敏度换取对强邻台的余量——在夜间拥挤的频段上，这是正确的设置。
settings-radio-front-end-attenuation-gain-0 = 前端衰减，以增益表示——0 dB 即无衰减。仅在 AGC 关闭时生效。
settings-radio-preamp = 前置放大器
settings-radio-hf-low-noise-amplifier-buys = HF 低噪声放大器。以互调为代价换取灵敏度，因此默认关闭——在真实天线上通常是正确的。
settings-radio-lna = LNA 开
settings-radio-frequency-calibration = 频率校准
settings-radio-parts-per-billion-receiver-s = 十亿分之一——这是此接收机自己的单位，比 RTL-SDR 使用的 ppm 精细一千倍。此处的任何内容都永远不会写入接收机的闪存：它只在本次会话中覆盖已存储的数值。
settings-radio-use-receiver-s-stored-value = 使用接收机已存储的数值
settings-radio-ppb = {" "}ppb
settings-radio-feed-dc-up-coax = 通过同轴电缆供直流
settings-radio-not-every-hf-has-one = 并非每台 HF+ 都有此功能；在没有它的接收机上，此项不起任何作用。
settings-radio-host-dsp = 主机 DSP
settings-radio-image-balancer-zero-if-offset = 镜像平衡器、零中频偏移以及微调振荡器。只有在需要查看原始硬件输出时才将其关闭——关闭后，镜像会在零中频速率上出现，而表盘仅能精确到最接近的千赫兹。
settings-radio-correct-image-fine-tune = 校正镜像并微调
settings-radio-copies-last-session-s-trace-2 = 将上一次会话的跟踪记录复制到剪贴板，用于错误报告：与接收机交换的每一条命令，以及采样数据流的前几个字节。
settings-radio-receive-only-no-soapysdr-no = 仅接收。无需 SoapySDR，也无需 libairspyhf。在合成器的下限以下，由主机振荡器完成调谐，这正是此接收机能够到达 VLF 的方式。接收机和采样率在“应用”后生效；其他设置在你更改时即生效。
settings-radio-not-yet-verified-against-real-3 = 尚未在实际硬件上验证。若出现异常，请发送诊断报告——它包含与接收机交换的每一条命令，以及采样数据流前几个字节解码为 I/Q 对的结果。
settings-radio-no-airspy-r2-mini-found = 未找到 Airspy R2 或 Mini——请按“重新扫描”
settings-radio-read-receiver = 从此接收机读取
settings-radio-r2-mini-offer-different-rates = R2 和 Mini 提供的速率不同，在打开其中一台之前无法区分
settings-radio-rate-you-get-receiver-s = 这是你将得到的速率。接收机的 ADC 以其两倍的速率运行——它对真实信号进行数字化，然后由 sdroxide 在主机上从中生成复数基带。
settings-radio-gain-curve = 增益曲线
settings-radio-step-along-curve-above-not = 沿上面曲线的一个步进，而不是分贝数——调谐器的 LNA、混频器和 VGA 会一起移动，每一步的价值取决于曲线和频段。0 为最安静的一端。
settings-radio-tuner-agc = 调谐器 AGC
settings-radio-lna-2 = LNA
settings-radio-hands-lna-tuner-s-own = 将 LNA 交给调谐器自身的环路控制，覆盖曲线设置。
settings-radio-mixer = 混频器
settings-radio-same-mixer-stage = 混频级同理。
settings-radio-loop-running-gain-slider-no = 环路运行时，增益滑块不再设置它所控制的级——环路会覆盖它。
settings-radio-dc-antenna-port = 天线端口的直流
settings-radio-t-12-bit-packing = 12 位打包
settings-radio-third-less-usb-traffic-leave = USB 流量减少三分之一。请保持开启：这是一台 USB 2.0 设备，最高速率为打包时 30 MB/s、未打包时 40 MB/s。在重新连接后生效。
settings-radio-dc-removal = 直流去除
settings-radio-remove-adc-s-offset = 去除 ADC 的偏移
settings-radio-turn-off-see-raw-hardware = 关闭它即可看到原始硬件输出。值得了解杂散的位置：偏移落在跨度的边缘而非中心，因为信号在途中被平移了采样率的四分之一。
settings-radio-every-command-exchanged-receiver-sample = 与接收机交换的每一条命令、采样率的计算过程，以及解码为 I/Q 的前几个采样。
settings-radio-receive-only-24-1800-mhz = 仅接收，24–1800 MHz。无需 SoapySDR，也无需 libairspy。接收机、采样率和打包在“应用”后生效；其他设置在你更改时即生效。
settings-radio-not-yet-verified-against-real-4 = 尚未在实际硬件上验证。若出现异常，请发送诊断报告——它包含与接收机交换的每一条命令、速率计算过程，以及解码为 I/Q 对的前几个采样。
settings-radio-no-hydrasdr-rfone-found-press = 未找到 HydraSDR RFOne——请按“重新扫描”
settings-radio-these-are-rates-receiver-turned = 这些就是该接收机结果所具有的速率
settings-radio-receiver-only-reports-three-these = 接收机只报告其中三种；其余的位于固件的备用表中，较旧的版本可能不包含它们
settings-radio-rf-input = 射频输入
settings-radio-antenna-sma-only-socket-bias = 天线 SMA——唯一后接偏置三通的插座。
settings-radio-cable-socket-no-bias-tee = 线缆插座。这里没有偏置三通；硬件只把它接在 ANT 上。
settings-radio-powers-active-antenna-preamp-down-2 = 通过同轴电缆为有源天线或前置放大器供电。只有 ANT 插座具有此功能——两个线缆端口是普通输入。
settings-radio-third-less-usb-traffic-leave-2 = USB 流量减少三分之一。请保持开启：这是一台 USB 2.0 设备，最高速率为打包时 36 MB/s、未打包时 48 MB/s。在重新连接后生效。
settings-radio-one-these-boards-1d50-60a1 = 其中一种板卡使用 1d50:60a1，这是 HydraSDR 原型机与 Airspy R2 和 Mini 共用的 USB 标识。sdroxide 会在打开后检查固件，若选错了接口（无论哪个方向），都会提示。
settings-radio-receive-only-24-1800-mhz-2 = 仅接收，24–1800 MHz。无需 SoapySDR，也无需 libhydrasdr。接收机、采样率和打包在“应用”后生效；其他设置在你更改时即生效。
settings-radio-re-list-fobos-sdrs-no = 重新列出 Fobos SDR。不会打开任何设备，因此在接收时按下也是安全的。
settings-radio-no-fobos-sdr-found-press = 未找到 Fobos SDR——请按“重新扫描”
settings-radio-input-2 = 输入
settings-radio-through-tuner-lna-vga-mixer = 经由调谐器：LNA、VGA，以及混频器自身的频率范围。
settings-radio-direct-sampling-hf1-antenna-input = 在 HF1 天线输入上进行直接采样——无调谐器，无 LNA/VGA 增益，完全由软件调谐。
settings-radio-direct-sampling-hf2-antenna-input = 在 HF2 天线输入上进行直接采样，与 HF1 相同。
settings-radio-both-real-adc-channels-once = 两个真实 ADC 通道同时工作，由下面的分集滤波器合并——用于将噪声源置零，或合并两条衰落路径。
settings-radio-direct-sampling-tuner-so-lna = 直接采样——调谐器（因此 LNA/VGA 增益）不在通路中。下面的速率是软件下变频器的目标，而非硬件设置，因此实际达到的速率可能与之不同。
settings-radio-receiver-s-own-reported-rates = 此接收机自己报告的速率
settings-radio-measured-one-real-unit-connect = 在一台实际设备上测得——连接此接收机并按“重新扫描”即可查看它自己的列表
settings-radio-clock-source = 时钟源
settings-radio-external-reference = 外部参考
settings-radio-diversity-taps = 分集抽头
settings-radio-taps = {" "}抽头
settings-radio-one-tap-gain-phase-null = 一个抽头是一个增益和一个相位——在一个频率上的零点，其两侧都会变差，这正是模拟移相器所能做到的全部。每增加一个抽头，滤波器就能均衡 HF1 与 HF2 之间路径差的一个采样周期，正是它把那个陷波变成一整段都安静的频段。
settings-radio-receive-only-rf-port-tuner = 仅接收。射频端口（调谐器）、HF1/HF2（直接采样，在软件中解码）以及 HF1+HF2（两者同时工作，由分集滤波器合并）均可使用。接收机、输入和采样率在“应用”后生效；增益、时钟源和抽头在你更改时即生效——选中此电台后，分集滤波器的模式、适应速率和保持设置位于主窗口自己的 DIV 框中。
settings-radio-no-hackrf-found-press-rescan = 未找到 HackRF——请按“重新扫描”
settings-radio-front-end-amplifier-8-db = 前端放大器，以 8 dB 为步进。这是改变灵敏度的一级，也是在真实天线上最先过载的一级。
settings-radio-baseband-amplifier-after-mixer-2 = 混频器之后的基带放大器，以 2 dB 为步进。遇到弱信号时，先调高它，再去动 LNA。
settings-radio-rf-amp = 射频放大
settings-radio-one-switch-shared-transmit-setting = 一个开关，与下面的发射设置共用——电台会应用其进入的方向所对应的那一项。在真实天线上，通常应保持关闭。
settings-radio-hackrf-pro-chooses-itself-three = HackRF Pro 会自行选择此项——即采样率的四分之三，并在 FPGA 中滤波——并忽略主机请求的任何设置。
settings-radio-leave-automatic-choosing-one-too = 保持为“自动”。选择过窄的一项不仅会使频段边缘变软——还会撤销使直流尖峰远离你的信号的本振偏移。
settings-radio-about-3-v-50-ma = 通过同轴电缆供给约 3 V、50 mA，用于有源天线或前置放大器。仅限 HackRF One 或 Pro；Jawbreaker 和 rad1o 没有此电路。
settings-radio-remove-dc-mirror-image = 去除直流和镜像
settings-radio-zero-if-radio-so-own = 这是一台零中频电台，因此其自身的本振泄漏位于跨度中部，而混频器的正交误差会在其上产生镜像。关闭此项即可看到原始硬件输出，这是区分驱动程序问题与 DSP 问题的快捷方法。
settings-radio-corrects-reference-oscillator = 校正参考振荡器。
settings-radio-transmit = 发射
settings-radio-transmitter = 发射机
settings-radio-off-default-while-off-backend = 默认关闭。在此关闭期间，后端不会发布任何发射通道，因此无法键控电台。在重新连接后生效。
settings-radio-tx-vga-gain = 发射 VGA 增益
settings-radio-transmit-driver-amplifier-drive-applied = 发射驱动放大器。驱动在此级之前以数字方式施加，因此请保持驱动较高，并在此处设置输出电平——反之，将驱动调低会使 DAC 以满量程的一小部分运行，并增加互调。
settings-radio-same-switch-receive-setting-above = 与上面接收设置相同的开关，在电台切换方向时应用。
settings-radio-transmit-armed-into-dummy-load = 发射已就绪。在完成测量之前，请接入假负载：HackRF 的谐波很强，需要为你所在的频段配备外部低通滤波器，而且它是半双工，因此每次通话期间接收都会停止。
settings-radio-every-command-exchanged-radio-session = 本次会话中与电台交换的每一条命令，按顺序排列，包括每次按下发射前后的序列。
settings-radio-dc-7-25-ghz-half = 直流 – 7.25 GHz，半双工。无需 SoapySDR、libusb 或 libhackrf。电台和采样率在“应用”后生效；其他设置在你更改时即生效。频段边缘由固件决定：HackRF 的规格为 1 MHz – 6 GHz（Pro 为 100 kHz），在此范围之外会被大幅衰减，但它确实可以调谐到那里。
settings-radio-ask-sdrplay-api-service-device = 向 SDRplay API 服务查询其设备列表。不会打开任何设备，因此在接收时按下也是安全的。
settings-radio-no-rsps-press-rescan-needs = 没有 RSP——请按“重新扫描”（需要 SDRplay API 服务）
settings-radio-both-tuners-running-api-fixes = 当两个调谐器都运行时，API 将 ADC 固定在 6 MHz，并从低中频返回 2 Msps——因此 2 Msps 是最宽的跨度，较窄的跨度则是对它进行抽取得到的。在“应用”后生效。
settings-radio-rates-below-2-msps-run = 低于 2 Msps 的速率会让 ADC 以 2 Msps 运行，并在服务中进行抽取。在“应用”后生效。
settings-radio-reduced-adc-resolution = {"  "}（降低的 ADC 分辨率）
settings-radio-if-bandwidth = 中频带宽
settings-radio-tuner-s-analog-filter-auto = 调谐器的模拟滤波器。“自动”会选择适合该采样率的最宽的一个。在“应用”后生效。
settings-radio-rsp-s-own-if-gain = RSP 自身的中频增益环路，由 API 服务运行。关闭则把中频增益滑块交还给你——这是用于测量和弱信号数字模式的设置。
settings-radio-agc-set-point = AGC 设定点
settings-radio-signal-level-loop-holds-adc = 环路将 ADC 保持在的信号电平。调低会为频道外的信号留出更多余量。它能降到多低取决于采样率——高于 8.064 Msps 时，变换器能提供的余量更少，高于 9.216 Msps 则更少。
settings-radio-rsp-s-native-gain-unit = RSP 的原生增益单位：20 dB 为最大增益（使用下面的扩展范围时为 0），59 dB 为最小。立即生效。AGC 运行时忽略此项——此时该数值由环路掌控，S 表显示的是它最终稳定的结果。
settings-radio-extended-if-range = 扩展中频范围
settings-radio-lets-if-gain-reduction-go = 允许中频增益衰减低于 20 dB，直至 0——即接收机最后的 20 dB 增益。关闭为 API 自身的默认值，也是普通收听的正确选择，因为该范围的底端正是 RSP 最容易过载的地方。在 LNA 已经为 0 的弱信号情况下，值得开启它。它还允许 AGC 设定点升至 0 dBFS。立即生效。
settings-radio-allow-if-gain-reduction-below = 允许中频增益衰减低于 20 dB
settings-radio-front-end-attenuation-steps-0 = 分步的前端衰减：0 为最大增益，每向上一步就多切入一级衰减。某些频段的步进较少——驱动程序会将其钳位，并保留你的选择，供你调回时使用；此处的滑轨止于电台所在频段所能接受的最大值。在另一个频段上做出的更高选择会被保留，并仍显示在其旁边。立即生效。
settings-radio-t-0-max-gain = 0 = 最大增益
settings-radio-reference-error-parts-per-million-2 = 参考误差，单位为百万分之一，由设备自身应用。立即生效。
settings-radio-radio-s-tuner = 此电台的调谐器
settings-radio-tuner = 调谐器
settings-radio-which-rspduo-s-two-tuners = 此电台收听 RSPduo 两个调谐器中的哪一个。另一个属于为它配置的那台电台——请给那一台设置相同的接收机和另一个调谐器。在“应用”后生效。
settings-radio-which-rspduo-s-two-tuners-2 = RSPduo 两个调谐器中，哪一个承载你正在收听的天线。另一个承载第二根天线。在“应用”后生效。
settings-radio-which-rspduo-s-two-tuners-3 = 运行 RSPduo 两个调谐器中的哪一个（一次一个）。在“应用”后生效。
settings-radio-applies-immediately = 立即生效。
settings-radio-fm-broadcast-notch = 调频广播陷波
settings-radio-hardware-notch-over-88-108 = 针对 88–108 MHz 调频广播频段的硬件陷波，用于当本地发射机使其他一切都过载时。立即生效。
settings-radio-dab-notch = DAB 陷波
settings-radio-hardware-notch-over-165-230 = 针对 165–230 MHz DAB 频段的硬件陷波。立即生效。
settings-radio-hdr-mode = HDR 模式
settings-radio-enable-below-2-mhz = 在 2 MHz 以下启用
settings-radio-rspdx-s-high-dynamic-range =
    RSPdx 用于 LF/MF 的高动态范围通路。立即生效。

    它目前尚不能工作：在已尝试过的那一台 RSPdx 上，开启它会使接收机静音——在所尝试的每一个中心频率、滤波器、端口、增益和速率下均是如此。该模式所需的东西并不在已公开的 API 范围之内，找到它是后续某人的任务。除非你就是那个正在查找的人，否则请保持关闭。

    已知的情况是：它并非跟随表盘的模式。该通路具有固定的模拟滤波器，只在下面滤波器所列出的中心频率上构建，因此即使其余部分都正常，调谐到其他任何地方也无法工作。
settings-radio-hdr-filter = HDR 滤波器
settings-radio-feed-4-7-v-dc = 通过同轴电缆供给约 4.7 V 直流
settings-radio-second-tuner = 第二个调谐器
settings-radio-rspduo-two-whole-tuners-one = RSPduo 是同一块板卡上的两个完整调谐器，由同一个参考时钟驱动。两者同时运行时，它们会在同一时刻、以同一时钟收听各自的跨度——这使两根天线之间保持相干，其相对相位由馈线决定而非偶然，并使这一对能够被合并。或者让它们各自独立：两个调谐器分别调谐，因此另一个可以在它自己的频段上作为第二台电台。
settings-radio-run-both-tuners = 运行两个调谐器
settings-radio-puts-rspduo-api-s-dual = 将 RSPduo 置于 API 的双调谐器模式，此时 ADC 固定在 6 MHz，最宽跨度为 2 Msps。在“应用”后生效，因为该模式是在设备打开时选定的。
settings-radio-used = 用于
settings-radio-what-other-tuner-takes-effect = 另一个调谐器用于什么。在“应用”后生效——与同时运行两者一样，这是在板卡打开时选定的。
settings-radio-add-second-radio-settings-radio = 添加第二台电台（设置 → 电台 → +），赋予它这同一个接收机以及 RSPduo 的另一个调谐器，并同样将其设为运行两个调谐器——无论哪台电台打开该板卡，都会将其置于双调谐器模式，因此两者都必须预期这一点。随后两者可以独立调谐：一个选项卡处理 HF，另一个处理 VHF，均来自同一块板卡。它们共用一个 ADC 时钟，因此无论打开板卡的是哪台电台所请求的采样率，两者都以该采样率运行，而且两者都不能发射。
settings-radio-what-do = 用途
settings-radio-lna-state-2 = 其 LNA 状态
settings-radio-second-tuner-s-own-front = 第二个调谐器自己的前端衰减。设置为使两根天线显示大致相同的底噪：整个系统都依赖于此项调整，因为合并时会按各分支的噪声为其加权；若第二个前端被驱动至过载，就会给滤波器一份失真的干扰副本——而失真的副本无法从未失真的副本中减去。立即生效。
settings-radio-if-gain-reduction-2 = 其中频增益衰减
settings-radio-second-tuner-s-if-gain = 第二个调谐器的中频增益，以 RSP 的原生单位表示：20 dB 为最大增益。AGC 运行时忽略此项——而滤波器需要的是稳定的增益，因此若要保持一个陷波，关闭 AGC 是值得的。
settings-radio-filter-length = 滤波器长度
settings-radio-one-tap-gain-phase-null-2 = 一个抽头是一个增益和一个相位——在一个频率上的零点，其两侧都会变差，这正是模拟移相器所能做到的全部。每增加一个抽头，滤波器就能均衡两根天线之间路径差的一个采样周期，正是它把那个陷波变成一整段都安静的频段。
settings-radio-adaptation-2 = 适应
settings-radio-slow-steady-left-converging-inside = 在左侧缓慢而稳定，在不到一秒内收敛；在右侧则明显地来回搜寻。先快速找到零点，再将其保持。
settings-radio-stop-filter-moving-reach-moment = 让滤波器停止移动。一旦出现零点就应使用它：若滤波器持续自适应，它会重新对准变得最响的任何东西，而在安静的频段上，那往往就是你正在收听的电台。
settings-radio-restart = 重新开始
settings-radio-zero-filter-find-null-again = 将滤波器归零，并重新找到零点。
settings-radio-nothing-here-can-tell-wanted = 这里没有任何东西能区分所需信号与不需要的信号——滤波器只知道两根天线的共同之处。在“消除”模式下，第二根天线应当只听到噪声源，并尽可能少地听到频段中的其他内容，否则它会尽职地把电台也消除掉。在“合并”模式下，两者都应听到同一个电台。零点有多深，每隔几秒便会写入日志。
settings-radio-not-yet-verified-against-rspduo = 尚未在 RSPduo 上验证：此处的双调谐器操作是依据 SDRplay 的 API 编写的，而非在硬件上测量的。
settings-radio-no-receiver-listed-check-against = 没有列出可用于核对的接收机——请按“重新扫描”，或阅读日志以了解实际打开的是哪一台。
settings-radio-receive-only-1-khz-2 = 仅接收，1 kHz–2 GHz。需要厂商的 SDRplay API 服务（sdrplay.com/api）——原始 RSP1 之后的 RSP 没有公开协议。设备、采样率、带宽和 RSPduo 调谐器在“应用”后生效；其他设置在你更改时即生效。
settings-radio-board = 板卡
settings-radio-first-one-found-2 = 找到的第一个
settings-radio-receive-chain = 接收通道
settings-radio-both-chains-tune-together-they = 两个通道一同调谐——它们共用一个合成器——但它们是位于不同插座上的独立前端。请选择你的天线所接的那一个；当其中一个通道做过 HF 匹配改装而另一个仍为原装时，这就是需要用到的设置。
settings-radio-receive-gain = 接收增益
settings-radio-one-combined-figure-which-limesuite = 一个合并的数值，由 LimeSuite 自己分配到 LNA、TIA 和 PGA 上。它只接受整数分贝，更细的数值会被截断。
settings-radio-receive-port = 接收端口
settings-radio-automatic-follows-frequency-lnal-low = “自动”会跟随频率：LNAL 在低频、LNAH 在高频——除非下方连接了 LimeRFE，它通过一根线缆接入一个插座，此时在所有频率上都是 LNAW。若你的插座不是这一个，请指明它接线所用的插座。该插座属于哪一个通道，由上面的选择器决定。
settings-radio-t-0-follows-sample-rate-worth = 0 跟随采样率。最好保持在那里：比跨度四分之一更窄的滤波器会悄悄撤销零中频本振偏移，使本振泄漏重新出现在你正在收听的信号之上。
settings-radio-following-rate = 跟随速率
settings-radio-corrections = 校正
settings-radio-host-iq-dc-correction = 主机 IQ / 直流校正
settings-radio-adaptive-image-dc-removal-side = 在本端进行自适应镜像和直流去除，叠加在芯片自身的校准之上。关闭它是一键区分驱动程序问题与 DSP 问题的方法。
settings-radio-calibrate-automatically = 自动校准
settings-radio-runs-chip-s-own-dc = 在电台打开时运行芯片自身的直流偏移和镜像校准，并在表盘于新频段或不同插座上稳定后再次运行。这些数值是在一个频率上测得的，在其他地方并不准确——而位于跨度中部的载波通常就是这种情况。每次约需一秒。
settings-radio-calibrate-now = 立即校准
settings-radio-stalls-receiver-better-part-second = 使接收机停顿将近一秒。
settings-radio-second-aerial = 第二根天线
settings-radio-socket = 其插座
settings-radio-same-first = 与第一个相同
settings-radio-gain-2 = 其增益
settings-radio-set-low-coupled-sample-your = 请将此项设为低。你自己发射机的耦合采样是一个强信号，而被驱动至压缩的反馈通道会错误地测量功放曲线——它会让校正学习到它自己的失真。请从最低处开始，并使用耦合器的衰减器。
settings-radio-set-so-both-aerials-show = 设置为使两根天线显示大致相同的底噪。整个系统都依赖于此调整：合并时会按各分支的噪声为其加权；若第二个通道被驱动至压缩，就会给滤波器一份失真的干扰副本——而失真的副本无法从未失真的副本中减去。
settings-radio-table-steps-2 = 表格步数
settings-radio-how-finely-correction-follows-amplifier = 校正对功放曲线的跟随精细程度。步数越多越能追踪更陡的拐点，但每一步都必须从落入其中的采样中学习——而语音幅度直方图的顶端很稀疏。对于 HF 功放实际具有的平滑曲线，三十二步已足够。更改它会重新开始校正。
settings-radio-how-hard-each-block-feedback = 每一块反馈对校正的推动力度。功放的曲线不会改变，因此无需急于求成——取中间值可把若干次通话的噪声平均掉。
settings-radio-keep-correction-curve-learned-clean = 保持校正不变。在一次干净的通话中学到的曲线值得保留，而到下一次通话时功放也不会有所改变。
settings-radio-forget-correction-learn-again = 忘记校正，重新学习。
settings-radio-directional-coupler-amplifier-s-output = 功放输出端的定向耦合器接入此通道，发射机会比较返回的信号与其本意发送的信号，然后发送该差值的反相——因此离开功放的信号是纯净的。对他人的通联，互调减少二十余分贝，而功放的功率保持不变。在反馈与发射对齐之前，校正保持为单位增益，因此未连接的耦合器不会造成任何损失；并且它永远不会要求变换器超过满量程，因此读数异常的反馈通路也不会过驱动任何东西。它的进展情况会在你发射时写入日志。
settings-radio-transmit-not-armed-so-there = 发射未就绪，因此此处没有需要校正的内容。
settings-radio-off-interface-publishes-no-transmit = 关闭时，接口不会发布任何发射通道，因此无法键控电台。
settings-radio-transmit-gain = 发射增益
settings-radio-transmit-port = 发射端口
settings-radio-board-s-two-transmit-sockets = 板卡的两个发射插座是两个不同的匹配网络，而不是接到同一网络上的两个插孔：BAND1（TX_1）覆盖 30 MHz 至 1.9 GHz，BAND2（TX_2）是 13 cm 端口。若从 BAND2 发射 23 cm 以下的业余频段，通话会进入一个几乎不让任何能量通过的匹配网络——每个设置都显示正常，驱动为你所设的值，而功率计则保持为零。将此项保持为“自动”，它便会跟随表盘。
settings-radio-limesdr-transmits-about-100-khz = LimeSDR 从约 100 kHz 发射至 3.8 GHz，自身不做任何滤波。请使用低通滤波器、合适的 LimeRFE 通道或假负载。
settings-radio-limerfe-front-end = LimeRFE 前端
settings-radio-connected = 连接方式
settings-radio-limerfe-s-own-micro-usb = LimeRFE 自身的 micro-USB 端口，而非电台的。9600 波特，由其固件固定。
settings-radio-receive-connector = 接收连接器
settings-radio-transmit-connector = 发射连接器
settings-radio-follow-dial = 跟随表盘
settings-radio-switch-filters-match-operating-frequency = 在任何射频出现之前，将滤波器切换为与工作频率相匹配。在同一频段内调谐，不会在控制链路上发送任何内容。
settings-radio-relays = 继电器
settings-radio-receive-attenuator = 接收衰减器
settings-radio-notch-filter = 陷波滤波器
settings-radio-fan = 风扇
settings-radio-worth-having-any-sustained-transmitting = 任何持续发射时都值得开启。
settings-radio-automatic-board-receives-switched-transmit = 在“自动”下，板卡处于接收状态，并在按下发射时切换为发射，松开时切回——无论采用哪种接线。其业余频道在两侧各有一个收发开关，夹着一个滤波器，因此若同时要求板卡两个方向，该开关会被接到发射机上，而接收将收不到任何东西。30 MHz 频道只能通过 J5 访问，这是一个同时用于两个方向的连接器；在它以上，从 J4 发射可使接收通路远离功放正在驱动的连接器。
settings-radio-limesdr-support-has-not-been = LimeSDR 支持尚未在硬件上验证。这是上一次会话的跟踪记录，用于问题报告。
settings-radio-gains-filters-corrections-antenna-sockets = 增益、滤波器、校正、天线插座以及每一项 LimeRFE 控件——其连接器、频段、继电器、衰减器、陷波和风扇——均立即生效。板卡、接收通道、采样率、发射就绪以及 LimeRFE 的连接在“应用”后生效。

## 设置窗口，电台选项卡：带数值填充的消息

settings-radio-fill-from = 从 { $name } 填充
settings-radio-decimation-auto = 自动（最接近的 { $khz } kHz）
settings-radio-decimation-stage-server = 第 { $stage } 级——服务器速率 ÷ { $divisor }
settings-radio-decimation-stage = 第 { $stage } 级——速率 ÷ { $divisor }
settings-radio-kiwi-receive-only = 仅接收——这是别人的天线。全景图是接收机以 I/Q 发送的约 12 kHz，它跟随表盘；其上方的频段视图是它自己的瀑布图，在其上调谐即会重新调谐接收机。连接期间约 { $rate } kB/s。按下“应用 / 重新连接”即可无需重启地切换。
settings-radio-connected-detail = 已连接：{ $detail }
settings-radio-failed = 失败：{ $error }
settings-radio-pluto-buffer = ≈ { $ms } ms，每次传输 { $kib } KiB
settings-radio-driver-settings = { $driver } 设置
settings-radio-rx888-coverage = 0–{ $mhz } MHz 覆盖范围，通过 USB 为 { $mbytes } MB/s
settings-radio-rx888-digitised = 在 { $mhz } MHz 数字化范围中
settings-radio-elad-16-bit = { $khz } kHz（16 位采样）
settings-radio-elad-pad = 输入 { $db } dB 衰减
settings-radio-hackrf-rx-preamp = 接收时 { $db } dB 前置放大
settings-radio-hackrf-tx-preamp = 发射时 { $db } dB 前置放大
settings-radio-device-not-found = { $device }（未找到）
settings-radio-lime-chain = 通道 { $n }（RX{ $n }_*）
settings-radio-lime-chain-tx = 通道 { $n }（RX{ $n }_* / TX{ $n }_*）
settings-radio-lime-second-chain = 板卡的另一条接收通道，位于 { $sockets } 插座上。它与第一条共用合成器，因此在同一瞬间收听相同的跨度——这正是它能承载第二根天线，或你自己发射机采样的原因。
settings-radio-tx-blocked = 发射被阻止：{ $reason }
settings-radio-rfe-rx-fallback = 在 { $port } 上接收时，这些频段会回退到未滤波的宽带通路：{ $bands }。
settings-radio-rfe-tx-fallback = 在 { $port } 上发射时，这些频段会回退到宽带通路——无频段功放，也无滤波：{ $bands }。

## 顶部栏和控制条（src/app/top_bar.rs）

topbar-how-far-one-press-moves = 按一次移动表盘的距离。点按即可进入下一个步进：10 Hz、100 Hz、500 Hz、1、2.5、5、9、10 和 25 kHz。在“设置 › UI”中可关闭此行。
topbar-volume-gain-agc-squelch-noise = 音量、增益、AGC、静噪、噪声控件，以及调频广播上的 RDS 读数
topbar-receiver = 接收机
topbar-vfo-b-split-rit-xit = VFO A/B、异频，以及 RIT/XIT 偏移
topbar-tuning = 调谐
topbar-diversity-filter-which-way-combines = 分集滤波器：以何种方式合并两根天线、适应速度，以及将其保持在当前位置
topbar-diversity = 分集
topbar-second-receiver-s-frequency-mode = 第二台接收机的频率、模式、滤波器和电平
topbar-sub-receiver = 副接收机
topbar-transceiver-s-own-aerial-socket = 收发信机自身的天线插座及其电源开关
topbar-radio = 电台
topbar-tune-voice-keyer-drive-mic = 调谐、语音键控器，以及驱动和话筒电平
topbar-transmit = 发射
topbar-tune = {" "}调谐{" "}
topbar-panadapter-two-layers-their-speeds = 全景图——其两个图层、它们的速度和细节、峰值保持——以及瀑布图对比度、FFT 大小和 skimmer
topbar-display = 显示
topbar-logbook-spots-awards-memories-scanner = 日志、信号、奖项、记忆信道、扫描器、设置和手册
topbar-system = 系统
topbar-linked = 已连接
topbar-not-linked = 未连接
topbar-bar-trace = 条形 / 轨迹
topbar-needle-bar-trace = 指针 / 条形 / 轨迹
topbar-swap-vfos = 交换 VFO
topbar-copy-b = 将 A 复制到 B
topbar-second-receiver-right-ear-tunes = 第二台接收机，在右耳中收听。它独立于 A/B 进行调谐——其控件出现在 SUB 模块中，其通带显示在瀑布图上。
topbar-band-plan = ，来自频段计划
topbar-repeater-shift-following-band-plan = 中继偏移：跟随频段计划，而该频率在频段计划中没有偏移——即单工
topbar-repeater-shift-transmit-above-below = 中继偏移：在表盘之上或之下发射，以使用中继台。现已为单工
topbar-repeater-shift = 中继偏移
topbar-auto = 自动
topbar-take-shift-band-plan-dial = 随表盘移动，从频段计划中获取偏移。它只在中继输出子频段内生效——在其他任何地方，它都会使电台保持单工，因此呼叫信道保持单工。
topbar-offset = 偏移
topbar-repeater-tone-ctcss-dcs-goes = 中继音：在语音下发出的 CTCSS/DCS、1750 Hz 脉冲，以及接收音静噪
topbar-transmit-tone = 发射音
topbar-set-here-sent-nfm-sub = 在此设置，以 NFM 发送——亚音频信令属于 FM 信道
topbar-normal = 正常
topbar-invert = 反相
topbar-bit-order-dcs-encoded-here = 此处 DCS 编码所用的位序是从标准转录而来的，从未在中继台上核对过——若无法打开，请尝试 CTCSS
topbar-t-1750-hz-burst = 1750 HZ 脉冲
topbar-every-over = 每次通话
topbar-open-burst-every-time-transmitter = 每当发射机按下时，都以脉冲开启
topbar-send = 发送
topbar-send-burst-now-keying-transmitter = 立即发送脉冲——若发射机尚未按下，则按下它以持续该脉冲的时长
topbar-nfm-only-burst-fm-repeater = 仅 NFM：脉冲即 FM 中继台的开门信号
topbar-receive-tone-squelch = 接收音静噪
topbar-require-receive-what-station-transmits = 在接收时要求本台所发射的音调
topbar-hide-docked-band-selector-click = 隐藏已停靠的频段选择器——再次点击即可重新显示
topbar-show-docked-band-selector = 显示已停靠的频段选择器
topbar-dock = 停靠
topbar-keep-band-mode-selector-open = 将频段和模式选择器保持在瀑布图旁边打开，而不是每次都关闭此弹出窗口
topbar-band-mode = 频段与模式
topbar-hide-band-chip-brings-back = 隐藏——频段标签可将其重新带回
topbar-undock = 取消停靠
topbar-return-selector-top-bar-popup = 将选择器返回顶部栏弹出窗口
topbar-dec-off = DEC 关
topbar-vol = 音量
topbar-counted-steps-not-decibels-what =
    {""}

    按步进计数，而非分贝：每一步的价值是接收机自己的事情，在 SDRplay 上它还取决于频段。数值越大，增益仍然越大。
topbar-gain = 增益
topbar-plutosdr-s-own-agc-setting =
    {""}

    PlutoSDR 自己的 AGC 正在设置此增益，因此移动滑块不起作用。在“设置 → 电台 → AGC”中将其设为手动，即可在此控制。
topbar-agc-hang-time-click-cycle = AGC 保持时间——点击循环切换：关 / 慢 / 中 / 快
topbar-radio-s-own-agc-click = 电台自身的 AGC——点击循环切换：慢 / 中 / 快。无法在此处将其关闭。
topbar-man = 手动
topbar-manual-audio-gain-used-while = 手动音频增益，在 AGC 关闭时使用。以关闭 AGC 时它所保持的电平为初始值。
topbar-radio-s-own-squelch-sent = 电台自身的静噪，通过控制链路发送。这是音频实际通过的闸门——软件静噪只会在电台已放行的内容上进一步关闭。
topbar-max-gain = 最大增益
topbar-manual-gain = 手动增益
topbar-noise-reduction = 降噪
topbar-impulse-noise-blanker = 脉冲噪声消隐器
topbar-auto-notch-cancel-constant-tone = 自动陷波：消除恒定音调成分（差拍干扰）
topbar-noise-reduction-off-click-switch = 降噪，关闭——点击即可开启，或在下面的各行中选择一个引擎
topbar-binaural-audio-not-while-sub = 双耳音频——副接收机占用右耳时不可用
topbar-binaural-audio-passband-spread-across = 双耳音频：通带分布在两只耳朵上，因此不同音调的信号来自不同方向，你所调谐的那一个会在两耳之间漂移。点击可恢复单声道
topbar-binaural-cw-spread-passband-across = 双耳 CW：将通带分布在两只耳朵上，使不同音调的信号来自不同方向——拥挤的通联不再只是一个挤在一起的音，而是分散在多个位置，调谐某个电台时它会在两耳之间漂移。在耳机上效果最佳
topbar-binaural-audio-spread-passband-across = 双耳音频：将通带分布在两只耳朵上。对于话音，噪声会分散在整个声像中，而电台则保持在中间，这样听上一个小时会更轻松——代价是话音本身也被分散开。在耳机上效果最佳
topbar-mute = 静音
topbar-record-audio-raw-i-q = 录制音频、原始 I/Q，或两者
topbar-wfm-stereo-forced-off-click = 调频立体声已强制关闭——点击以恢复自动立体声
topbar-wfm-stereo-pilot-locked-click = 调频立体声：导频已锁定。点击可强制单声道
topbar-wfm-stereo-automatic-no-pilot = 调频立体声：自动，此电台没有导频
topbar-rds-data-arriving-click-see = 正在接收 RDS 数据——点击查看
topbar-rds-no-data-station-click = RDS——此电台没有数据。仍可点击查看解码器的诊断信息
topbar-drm-decoding-click-details = 正在解码 DRM——点击查看详情
topbar-drm-click-decoder-s-state = DRM——点击查看解码器的状态
topbar-hd-radio-decoding-click-details = 正在解码 HD Radio——点击查看详情
topbar-hd-radio-click-decoder-s = HD Radio——点击查看解码器的状态
topbar-presets = 预设
topbar-preset-right-click = 右键单击以更改此预设
topbar-preset-edit = 编辑预设
topbar-preset-label = 标签
topbar-preset-label-hint = 按钮上显示的文字，最多 8 个字符。
topbar-preset-lo = 下限
topbar-preset-hi = 上限
topbar-preset-use-current = 使用当前值
topbar-preset-use-current-hint = 采用接收机当前滤波器的边缘
topbar-preset-save = 保存
topbar-preset-reset = 重置
topbar-preset-reset-hint = 恢复此模式的内置预设
topbar-preset-cancel = 取消
topbar-width-per-sideband = 每个边带的宽度
topbar-width = 宽度
topbar-passband-width-hertz-type-drag = 通带宽度，单位为赫兹——可直接输入，也可拖动。点击输入框即可输入精确数值。
topbar-edges = 边缘
topbar-low-edge-hz-carrier = 低端边缘，距载波的赫兹数
topbar-high-edge-hz-carrier = 高端边缘，距载波的赫兹数
topbar-noise-reduction-voice-off-click = 降噪（话音），关闭——点击以选择引擎
topbar-record-audio = 录制音频
topbar-mp3 = ● MP3
topbar-what-you-hear-mp3-receiver = 你听到的内容，以 MP3 格式保存：接收机在一个声道，你自己的发射在另一个声道。这就是一次通联的录音。
topbar-mono = 单声道
topbar-mixed-one-channel-click-two = 混合为一个声道——点击切换为两个声道
topbar-two-channels-rx-left-tx = 两个声道：接收在左，发射在右——点击切换为混合的一个声道
topbar-quick-clip = 快速剪辑
topbar-stop-after = 之后停止
topbar-no-stop = 不停止
topbar-auto-record = 自动录制
topbar-set-squelch-auto-record-needs = 请设置静噪——自动录制需要它来判断静音
topbar-auto-record-needs-sdr-front = 自动录制需要带有软件静噪的 SDR 前端
topbar-stop-following-squelch = 停止跟随静噪
topbar-waiting-signal = 等待信号
topbar-record-spectrum = 录制频谱
topbar-i-q-wav = ● I/Q WAV
topbar-i-q-wav-2 = I/Q WAV
topbar-raw-spectrum-receiver-delivering-stereo = 接收机输出的原始频谱，保存为立体声 32 位浮点 WAV（超过 4 GB 时为 RF64），SDR#、SDRuno、HDSDR 和 SDRangel 都能打开它——sdroxide 自身也能用 --file 回放它。文件很大：每个采样 8 字节。
topbar-radio-hands-over-demodulated-audio = 此电台输出的是解调后的音频，因此没有 I/Q 可供录制。
topbar-what-capture-costs-sample-rate = 在此采样率下录制的开销
topbar-engine = 引擎
topbar-no-noise-reduction-decoders-never = 无降噪——解码器本来就从未看到它
topbar-strength = 强度
topbar-tone = 音调
topbar-ctcss-dcs-no-sub-audible = CTCSS / DCS——此信号上没有亚音频音调
topbar-tone-squelch = 音调静噪
topbar-carrier-squelch-open-any-signal = 载波静噪：任何信号都会打开
topbar-require-tone-currently-being-received = 要求当前正在接收的音调
topbar-ctcss = CTCSS
topbar-any-dcs = 任意 DCS
topbar-open-any-dcs-coded-signal = 任何带 DCS 编码的信号都会打开。它所携带的 104 个编码中具体是哪一个，无法在此可靠读取，因此没有更细的选项可选
topbar-which-socket-back-radio-receiving =
    电台在背面的哪个插座上接收——即其自身的 ANT 命令，与前面板上 ANT 按钮的设置相同。点击可切换到下一个。

    该选择按频段记忆，表盘下次进入该频段时会恢复。
topbar-radio-s-separate-receiving-antenna =
    电台独立的接收天线，切入或切出接收通路——即其自身的 RX ANT 设置。无论如何，主插座上的天线都保留用于发射。

    电台自己按频段记忆此设置，因此 sdroxide 会在每次频段变化后读回它，而不是恢复它最后一次见到的值：只有点击此处才会改变它。
topbar-radio-s-own-power-switch =
    电台自身的电源开关，通过控制链路操作，也是本程序中唯一真正的开关——而非 sdroxide 的“连接”开关，后者只关闭 sdroxide 这一端，电台仍在运行。

    要让“开”的指令能够生效，电台的控制端在关闭期间必须保持唤醒：即局域网上的“网络控制”，或在前面板关机、但仍由市电供电的机器上的 CI-V 端口。
topbar-switch-radio = 开启电台
topbar-switch-radio-off-audio-meters = 关闭电台。音频和表头随之停止。
topbar-what-second-aerial-click-swap =
    第二根天线的用途——点击即可切换。

    “消除”会在增益、相位和延迟上将其从第一根天线中减去，使本地噪声源在两者上都对齐：这是降噪移相器的 DSP 形式。“合并”则以相互增强的相位把两者相加，并按各自的接收质量加权——即分集接收，用于填补衰落。
topbar-hold = 保持
topbar-stop-filter-moving-reach-moment = 让滤波器停止移动。一旦出现零点就应使用它：若滤波器持续自适应，它会重新对准变得最响的任何东西，而在安静的频段上，那往往就是你正在收听的电台。
topbar-restart = 重新开始
topbar-zero-filter-find-null-again = 将滤波器归零，并重新找到零点。
topbar-adapt = 自适应
topbar-how-fast-filter-chases-slow = 滤波器追踪的速度：左侧缓慢而稳定，在不到一秒内收敛；右侧则明显地来回搜寻。先快速找到零点，然后将其保持。
topbar-where-sub-receiver-listens-shift = 副接收机收听的位置。按住 Shift 点击瀑布图，或在副接收机的通带内拖动，即可移动它。
topbar-dial = ←表盘
topbar-move-sub-receiver-main-dial = 将副接收机移动到主表盘
topbar-dial-2 = 表盘→
topbar-move-main-dial-sub-receiver = 将主表盘移动到副接收机
topbar-filter = 滤波器
topbar-sub-receiver-passband-edges-hz = 副接收机通带边缘，单位为赫兹
topbar-sub-receiver-level-plays-right = 副接收机电平（它在右耳播放）
topbar-ptt = {" "}PTT{" "}
topbar-voice-keyer-record-transmit-stored = 语音键控器：录制并发射已存储的消息
topbar-drive = 驱动
topbar-tune-2 = 调谐
topbar-tx-audio-processing-microphone-click = 发射音频正在处理话筒——点击打开
topbar-click-open-tx-audio-gate = 点击打开发射音频：话筒的噪声门、均衡器、压缩器和限幅器
topbar-mic = 话筒
topbar-cessb = CESSB
topbar-microphone-gain = 话筒增益
topbar-deviation-how-far-mode-s =
    频偏：此模式的脉冲使自行调制它的电台摆动多远。FM 发射机把音频电平转换为频率摆幅，而且没有 ALC 来制约它，因此把满量程送入一个设为话音的数据输入，会造成过度频偏——在收听者听来完全正常，却不能为任何人解码。

    按模式保存，因此为 1200 波特分组设置的频偏绝不会用到 FT8 上。
topbar-transmit-audio-how-hard-mode =
    发射音频：此模式以多大力度驱动自行调制我们所发送内容的电台的调制器——例如声卡上的 CAT 电台、FLEX，或网络端口上的 Icom。将其调低，直到电台的 ALC 几乎不动，然后在电台上设置功率；恒包络数字模式下 ALC 过度工作，正是造成频谱扩散的原因。

    驱动并非此控件：在这些电台上，驱动作用于电台的功率寄存器，从不触及其音频。

    按模式保存——FT8、RTTY、PSK 和 MCW 各自保存自己的设置。
topbar-tx-audio = 发射音频
topbar-cw-psk-rtty-skimmers-decode = CW / PSK / RTTY skimmer——在整个频段上解码信号，并在瀑布图上标注它们
topbar-skimmers = SKIMMER
topbar-run-skimmer = 运行此 skimmer
topbar-minimum-snr-decoded-signal-needs = 解码信号被标注所需的最低信噪比
topbar-needs-wideband-iq-source = 需要宽带 IQ 信源
topbar-cw-decoder = CW 解码器
topbar-how-many-signals-model-reads = 模型一次读取多少个信号。其余信号保留标记，但不附带文字。
topbar-spots = 信号列表
topbar-save-skimmer-s-spot-list = 将 skimmer 的信号列表保存到文件
topbar-show-hide-full-band-waterfall = 显示/隐藏全景图上方的全频段瀑布图条带——即此接收机能一次看到的全部内容
topbar-spectrum-waterfall-either-layer-both = 频谱和瀑布图——任选一层、两层或都不选。两者都显示时亮起。
topbar-spectrum = 频谱
topbar-show-spectrum = 显示频谱
topbar-draw-spectrum-line-across-top = 在全景图顶部绘制频谱线。关闭后，瀑布图占据全部高度。
topbar-peak-hold = 峰值保持
topbar-trace-highest-level-each-column = 在实时曲线之上追踪每一列所达到的最高电平，然后逐渐回落——即你看向别处时频段曾出现过的情况
topbar-draw-spectrum-receding-surface-instead = 以后退的曲面而非平面线绘制频谱：最新的频谱位于前排，之前的频谱则向远处流去。频段的最近几秒呈现为一幅景观——时有时无的载波是一道山脊，而不是抽动的线。网格和峰值保持属于平面线，不会绘制在其上。
topbar-how-quickly-spectrum-line-follows = 频谱线跟随频段的速度。越慢，相邻帧之间的平均越多：线条更稳定，而静止足够久的弱载波也能读出来。瀑布图不受其影响——无论如何，这些行都会获得每一帧。
topbar-waterfall = 瀑布图
topbar-show-waterfall = 显示瀑布图
topbar-draw-scrolling-waterfall-below-spectrum = 在频谱下方绘制滚动的瀑布图。关闭后，频谱线占据全部高度。
topbar-how-fast-waterfall-scrolls-lines = 瀑布图每秒滚动的行数：慢速 5、中速 28、快速 56、更快 112、最快 224。引擎自己计时，因此两个最快档是真正的细节，而不是同一行画了两遍——只要接收机能够供给：一行永远只能显示一次变换，而较窄的前端每秒只能产生几十行。它们会占用历史记录，因为瀑布图保存固定数量的行——中速时为 73 秒，最快时为 9 秒。
topbar-decode-labels = 解码标签
topbar-mark-every-decoded-station-waterfall = 在瀑布图上，以其被收听到的频率标记每一个已解码的电台及其呼号——适用于 FT8、FT4 和其他分时隙模式。良好的开端是每两分钟在整个跨度上放置三十个标签，覆盖在你正在阅读的轨迹之上；关闭它们，瀑布图就只是频段本身。旁边的解码列表仍然包含每一个呼号。
topbar-switch-3d-set = 请先开启 3D 以设置此项
topbar-how-3d-spectrum-drawn-only = 3D 频谱如何绘制。只有形状不同——两种呈现方式显示的都是相同的频谱，而且都会遮挡其后方的内容，因此前方的强信号会矗立在它所覆盖的频段之上。
topbar-lines = 线条
topbar-one-trace-per-remembered-spectrum = 每个记忆的频谱一条轨迹，颜色与平面线相同，并遮挡其后的轨迹。只显示形状而不显示电平——当你需要的是形状时，应选择此读法。它比实心模式的开销略大而非更小，因为它先绘制相同的曲面，然后在其上描绘每一个峰顶。
topbar-solid = 实心
topbar-filled-surface-coloured-waterfall-s = 一个由瀑布图调色板着色的填充曲面，因此电平既体现在颜色上，也体现在高度上，全景图的两半对强信号的样子看法一致。可在“设置 › 显示”中更改调色板。
topbar-how-fast-3d-spectrum-flows = 3D 频谱离你远去的速度，单位为每秒行数：慢速 6、中速 12、快速 24、更快 48。曲面的深度为固定的行数，因此这也决定了它保存的时间长度。越慢记忆越长，曲面则缓慢爬行；越快则记忆越短，但移动得更快。瀑布图有其自己的滚动速率，见下文。
topbar-how-many-columns-panadapter-waterfall = 全景图及其瀑布图绘制的列数。“自动”会读取此计算机的渲染器和全景图的大小，并选择它能承载的最大值——4K 屏幕需要 4096。每一列在每一帧中都占一个字节，因此连接到远程电台站点的客户端需要为其链路上的细节付出代价：4096 列在 60 fps 下约为每秒四分之一兆字节，是标准宽度的两倍。
topbar-read-machine-screen-take-most = 读取此计算机和此屏幕，取它们能承载的最大值，并在它们变化时随之调整
topbar-width-force-right-now-whatever = 当前生效的宽度，无论上面一行如何要求
topbar-keep-floor-ceiling-set-best = 保持为获得最佳瀑布图对比度而设置的底噪/顶端——在频段变化、平移或缩放之后，以及电平漂移时，会缓慢回到位置。开启它可立即适配；关闭它则使电平保持在你设置的位置。
topbar-keep-tuned-frequency-middle-panadapter = 将调谐频率保持在全景图中央：窗口会在表盘下滑动，而不是在你调谐到边缘之外时图像整段跳动。开启它可立即居中；关闭它则可随意平移和缩放。
topbar-spectrum-floor-ceiling-fft-size = 频谱底噪/顶端和 FFT 大小
topbar-level-drawn-bottom-spectrum-darkest = 绘制在频谱底部、并作为瀑布图最暗颜色的电平。将其调高，直到底噪刚好变暗。设置此项会关闭“适配”。
topbar-ceil = 顶端{" "}
topbar-level-drawn-top-bring-down = 绘制在顶部的电平。将其调低，直到你关心的最强信号达到满色。设置此项会关闭“适配”。
topbar-fft-size = FFT 大小
topbar-flip = 翻转
topbar-scroll-waterfall-upwards-newest-row = 使瀑布图向上滚动——最新的行在底部（V）
topbar-logbook-all-qsos-digital-manual = 日志——所有通联（数字 + 手动）
topbar-live-spots-dx-cluster-pota = 实时信号——DX 集群、POTA、SOTA、PSK Reporter
topbar-award-tracking-dxcc-was-waz = 奖项追踪——DXCC / WAS / WAZ / 网格
topbar-band-conditions-published-forecast-beside = 频段条件——公布的预报，旁边是每个频段上实际听到的情况
topbar-satellite-qo-100-beacon-calibration = 卫星——QO-100 信标校准正在运行
topbar-satellite-doppler-tracking-qo-100 = 卫星——多普勒跟踪，以及 QO-100 信标校准
topbar-ism-band-devices-decoding-now = ISM 频段设备——正在解码，无论此窗口是否打开。在窗口内使用 DECODING 将其关闭。
topbar-ism-band-devices-weather-sensors = ISM 频段设备——你周围听到的气象传感器、仪表和家庭自动化设备
topbar-public-sdrs-internet-browse-kiwisdr = 互联网上的公共 SDR——浏览 KiwiSDR 和 SpyServer 目录，并将其中之一作为电台打开
topbar-winlink-radio-email = Winlink 无线电邮件
topbar-memory-channels = 记忆信道
topbar-scanner-stopped-signal = 扫描器——停在一个信号上
topbar-scanner-running = 扫描器——运行中
topbar-scan-memory-channels-frequency-range = 扫描记忆信道或频率范围
topbar-hfdl-ground-network-decode-log = HFDL 地面网络——解码日志和飞机地图，位于瀑布图下方。在面板内使用 LISTEN 开启解码器。
topbar-hfdl-ground-network-decoding-now = HFDL 地面网络——正在解码。请打开面板。
topbar-hfdl-ground-network-aircraft-shortwave = HFDL 地面网络——飞机短波数据链，一次收听一个信道
topbar-grid-tracker-worked-maidenhead-squares = 网格追踪器——地图上已通联的梅登海德方格，以及已听到的内容
topbar-amplifier-set-up-hardrock-under = 功放——请在“设置 → 功放”下设置 Hardrock
topbar-settings-device-gains-antennas-audio = 设置——设备增益、天线、音频设备
topbar-user-manual-f1 = 用户手册（F1）
topbar-range-was-typed-settings-radio = 该范围是在“设置 ▸ 电台 ▸ RX 范围”中输入的，而不是由电台报告的。请将其加宽，或清空输入框以使用设备自行报告的范围。
topbar-what-device-reports-about-itself = 这是设备自行报告的内容。如果电台确实覆盖此频段，请在“设置 ▸ 电台 ▸ RX 范围”中填写其实际范围。
topbar-band = 频段
topbar-recent = 最近
topbar-recent-empty =
    在一处停留 3 秒后记录
topbar-mode = 模式
topbar-digital = 数字

## 顶部栏：系统框中的窗口标签。

topbar-chip-log = 日志
topbar-chip-spots = 信号
topbar-chip-awards = 奖项
topbar-chip-bands = 频段
topbar-chip-sat = 卫星
topbar-chip-ism = ISM
topbar-chip-public-sdr = 公共 SDR
topbar-chip-mail = 邮件
topbar-chip-mem = 记忆
topbar-chip-scan = 扫描
topbar-chip-hfdl = HFDL
topbar-chip-grid = 网格
topbar-chip-amp = 功放
topbar-chip-settings = ⚙ 设置
topbar-chip-help = ? 帮助

## 顶部栏：模式默认值标签。

topbar-changed-agc = AGC
topbar-changed-squelch = 静噪
topbar-changed-auto-notch = 自动陷波
topbar-changed-stereo = 立体声
topbar-changed-binaural = 双耳
# $mode 是模式名称（USB、CW……）；$changed 列出与其默认值不同的项目。
topbar-mode-defaults =
    返回 { $mode } 的默认值。已更改的项目：{ $changed }。

    点击即可将其恢复。{ $mode } 自身的数值会回来，你在此处设置的内容会被忘记。

## 设置窗口，电台选项卡：带数值的较长消息

settings-radio-no-such-baud = 此电台没有 { $baud } 波特的设置——将改用 { $default }
settings-radio-icom-scope-hint =
    通过 CI-V 链路传输电台自身的示波器扫描，并将其绘制为全景图——对于一台解调音频的电台，这是它能提供的唯一频段画面，因为它发送的音频本身就是已经过其滤波器的结果。

    需要为此进行电台设置：将 CI-V USB Baud Rate 设为 115200，并将 CI-V USB Port 设为“Unlink from {"["}REMOTE]”（两者都在 SET > Connectors > CI-V 下），并将上面的波特率设为与之相符——低于 { $min_baud } 时，扫描无法通过链路传输，示波器将保持关闭。

    这些扫描与电台自己的声卡共用其内部 USB 总线。若示波器开启时接收音频开始断续，应首先尝试关闭此框。
settings-radio-rx-tx-words = { $rx } / 发射 { $tx }
settings-radio-soapy-native = sdroxide 直接支持上面的硬件：{ $names }。选择上面的该接口可获得其自身的设置——SoapySDR 无法表达的增益级、滤波器和陷波——并且无需 SoapySDR 模块。
settings-radio-rx888-width-hint = 全景图一次显示数字化频谱的多少。整个 DSP 链路都以此宽度运行，因此越宽则 CPU 开销成比例增加——1/2 即瀑布图中的整个频段，计算量相当可观。在 VHF 分界点（{ $crossover } MHz）以上，全景图看的不是天线，而是调谐器的中频，它宽 8 MHz，固定在 { $if_centre } MHz——因此任何宽于 { $widest } 的设置都无法将其居中，多出的宽度只是信号旁边的空白频谱，而并非更多的信号。
settings-radio-sdrplay-serial-missing = 序列号 { $serial } 不在 SDRplay 服务报告的接收机之中。请选择列出的某一台接收机，或重新插拔此设备后按“重新扫描”。
settings-radio-hdr-filter-hint =
    位于 HDR 通路之前的模拟滤波器——与接收机自身的带宽是不同的控件。每个设置都只在固定的一组中心频率上构建，在其他地方则不起作用；此处的滤波器是在 { $centres } 上构建的。立即生效。

    该通路本身尚不能工作——见上面的开关——因此这里选择的滤波器，是为目前无声的模式所准备的。
settings-radio-rspduo-only = 只有 RSPduo 具有第二个调谐器，而此处所选的接收机是 { $model }，因此此项不会起任何作用。
settings-radio-lime-tx-gain-low = 发射增益为 { $db } dB，处于其范围的最低端——板卡只输出几微瓦，无论其后接的是什么，功率计上都会显示为零。这是默认值，以使已就绪的发射机不会让你措手不及；在按下发射之前请将其调高。

## 设置窗口，常规：SWR

settings-general-swr-tune-limit = （调谐：{ $limit }:1）

## 顶部栏和控制条：带数值的消息

# $reading 是电表自身的悬停文字；$faces 列出点击所循环切换的表盘面。
topbar-smeter-hover =
    { $reading }

    点击可循环切换电表表盘：{ $faces }
topbar-step-down = 降低 { $step }
topbar-step-up = 增加 { $step }
# $auto 为 topbar-band-plan 或为空。
topbar-repeater-shift-hover = 中继偏移 { $shift } — 在 { $rx } 上接收，在 { $tx } MHz 上发射{ $auto }
topbar-tone-and-burst = 在语音下发射 { $tone }，并在每次通话开始时发送一个 { $ms } ms 的 1750 Hz 脉冲
topbar-tone-under-voice = 在语音下发射 { $tone }
topbar-burst-every-over = 每次通话开始时发送一个 { $ms } ms 的 1750 Hz 脉冲
topbar-match-tx = 与发射匹配（{ $tone }）
topbar-decimation-on = 前端抽取：接收机只获得此电台所传输的 { $msps } Msps 中央的 1/{ $n }——{ $span }——其余部分在到达接收机之前即被丢弃。
topbar-decimation-off = 前端抽取，关闭：接收机看到的是此电台所传输的整个 { $span }。
topbar-decimation-hint =
    { $what }

    跨度越窄，瀑布图分辨率越精细，底噪越低（每减半 3 dB），CPU 开销也越小——代价是丢失其两侧的频段。表盘仍可调谐到任何地方；电台会移动其本振以跟随。

    点击循环切换：关闭 / 2 / 4 … { $max }。
topbar-rx-gain-hint = 前端接收增益（{ $stage }）。过大会使接收机的 ADC 削波，并把杂散信号涂抹到整个频段上；过小则接收机会变聋。
topbar-rx-gain-stages =
    {""}

    此电台有 { $n } 级接收增益——其余位于“设置 → 设备”中。
topbar-passband-now =
    {""}

    通带目前为 { $dbfs } dBFS。
# $level 为 topbar-passband-now 或为空。
topbar-squelch-hint =
    以 dBFS 为单位，在接收通带中将低于此功率的音频关闭。向左为打开。{ $level } 请将其设在噪声之上、信号之下。

    若数据流到达时已带有电台自身的 AGC（例如 Icom 的 12 kHz IF），它在此刻度上的位置远高于 SDR 的原始基带，这正是刻度会达到满量程的原因。
topbar-nr-cycle-hint =
    降噪：{ $strength } — { $engine }

    点击循环切换强度：低 / 中 / 高 / 关。引擎位于下面的各行中。
topbar-recording-both = 正在录制 { $audio } 和 { $iq }
topbar-recording-audio = 正在将音频录制到 { $audio }
topbar-recording-iq = 正在将 I/Q 录制到 { $iq }
topbar-auto-record-armed = 自动录制已就绪：每次发射一个文件，静音 { $secs } 秒后关闭
topbar-rds-hover = RDS：{ $ps }。点击查看电台的数据
topbar-drm-hover = DRM：{ $service }。点击查看此广播的详细信息
topbar-drm-no-codec = DRM：{ $summary } — 已锁定，但其 { $codec } 音频无法在此解码。点击查看缺少什么
topbar-drm-state = DRM：{ $summary } — 点击查看解码器的状态
topbar-hd-hover = HD Radio：{ $station }。点击查看此广播的详细信息
topbar-hd-state = HD Radio：{ $summary } — 点击查看解码器的状态
topbar-binaural-preset = 每只耳朵 { $preset } — { $lo } … { $hi } Hz，两个边带
topbar-nr-hover =
    降噪：{ $strength } — { $engine }

    点击更改引擎或强度
topbar-clipping = 剪辑 { $span } — 再次按下即可重新开始
topbar-record-clip = 录制 { $span } 并停止——作为接收报告附件的剪辑
topbar-starting-clip = 正在开始 { $span } 的剪辑……
topbar-stop-recording-after = 在 { $minutes } 分钟后停止 MP3 录制
topbar-stops-in = 将在 { $time } 后停止
topbar-auto-record-hint = 将每次发射录制到其自己的文件中，静音 { $secs } 秒后关闭
topbar-recording-silence = 正在录制 · 静音 { $secs } 秒（在 { $hold } 时关闭）
topbar-tone-open = 正在接收 { $tone }，即音调静噪——音频已打开
topbar-tone-closed = 正在接收 { $heard }，但音调静噪需要 { $wanted }——音频保持关闭
topbar-tone-heard = 正在接收 CTCSS/DCS { $tone }
topbar-tone-nothing = 音调静噪 { $tone }：没有接收到与之匹配的内容
topbar-tone-use = 使用 { $tone }
topbar-sockets = 插座：{ $sockets }
topbar-voice-keyer-tx = 正在发射 { $slot } — 点击打开语音键控器
topbar-drive-uncal-gen = 此频率不属于任何业余频段，因此未经校准，设置会完整地作用于发射机。
topbar-drive-uncal-band = { $band } 未经校准，因此设置会完整地作用于发射机。
# $trim 带符号（+1.5）；$diff 为其大小；$more 为“less”或“more”。
topbar-drive-cal-gen = 此频率不属于任何业余频段，经校准为 { $trim } dB，因此同样的设置在此处比未经校准的频段{ $more ->
        [less] 低
       *[more] 高
    } { $diff } dB 发射出去。
topbar-drive-cal-band = { $band } 经校准为 { $trim } dB，因此同样的设置在此处比未经校准的频段{ $more ->
        [less] 低
       *[more] 高
    } { $diff } dB 发射出去。
topbar-drive-hover =
    发射机被驱动的力度。在 sdroxide 自行调制的电台上，它缩放的是已调制的采样；在带有自身功率控制的电台上，它是请求电台输出的额定功率的比例。

    { $trim }

    设置 → 电台 → 分频段的发射驱动。
topbar-cessb-hover =
    受控包络 SSB：同样的峰值下平均功率更大，且没有频谱扩散——在对方听来响度增加约三到四分贝。这是话音被推入处理器的分贝数；0 为关闭，6 是合理的首次尝试。它不影响驱动。

    当前：{ $now }
topbar-waterfall-rate = 每秒 { $rows } 行——{ $seconds } 秒的频段
topbar-detail-auto-columns = { $n } 列，即“自动”在此处会选择的数值
topbar-n-columns = { $n } 列
topbar-fft-bins-whole = 整个数据流上 { $n } 个频点——每列一个，或更细
topbar-fft-bins-pooled = { $n } 个频点汇集到全景图的 { $cols } 列中：信号更锐利，但并非更多信号
topbar-sat-locked = 卫星——已锁定 { $name }
topbar-amp-in-line = { $model } — 已接入线路（PTT）
topbar-amp-bypassed = { $model } — 已旁路（关）
topbar-amp-not-answering = { $model } — 无应答
topbar-filter-edges = 距载波 { $lo } … { $hi } Hz
topbar-filter-isb = 接收滤波器：每只耳朵 { $width } Hz（{ $edges }）。ISB 的两个边带是各自独立的发射，因此此宽度是其中一个边带的宽度。
topbar-filter-width = 接收滤波器：宽 { $width } Hz（{ $edges }）。
topbar-filter-hint =
    { $what }

    点击可查看宽度、两个边缘以及此模式的预设值（以数字表示）——全景图上的手柄只能凭目测放置通带，精确的数值需在此输入。
topbar-band-outside-range =
    { $band } 超出此电台的接收范围（{ $ranges } MHz）。

    { $why }
# $time 为 "day" 或 "night"。
topbar-band-forecast = { $band }：{ $verdict }（{ $time ->
        [day] 白天
       *[night] 夜间
    }）——由 HAMQSL.com 根据太阳指数预报，而非你自己路径的测量。

## 顶部栏：简短词语和标题

topbar-squelch-open = 打开
topbar-off = 关
topbar-recording = 录制中
topbar-sql = SQL
topbar-stations = 电台
topbar-caption-reaction = 响应
topbar-caption-scroll = 滚动
topbar-caption-surface = 曲面
topbar-caption-flow = 流动
topbar-caption-detail = 细节
topbar-floor = 底噪

## 数字模式面板标签（src/app/panels/mod.rs，pane_label）。

pane-aircraft = 飞机
pane-chart = 图表
pane-chat = 聊天
pane-decodes = 解码
pane-files = 文件
pane-heard = 收听
pane-image = 图像
pane-map = 地图
pane-messages = 消息
pane-monitor = 监视
pane-panel = 面板
pane-qso = 通联
pane-reading = 阅读
pane-receive = 接收
pane-saved = 已保存
pane-send = 发送
pane-spots = 信号
pane-stations = 电台
pane-status = 状态
pane-terminal = 终端
pane-text = 文本
pane-traffic = 流量
pane-vessels = 船舶
pane-wfall = 瀑布

## 数字模式面板，共用部分（src/app/panels/mod.rs）

panels-save = {" "}保存{" "}
panels-nothing-decoded-save = 没有可保存的解码内容
panels-clear-rx = {" "}清空接收{" "}
panels-empty-receive-window-nothing-air = 清空接收窗口。空中的任何内容都不会停止。
panels-nothing-received-clear = 没有可清空的接收内容
panels-save-what-panel-has-decoded = 将此面板解码的内容保存到文件
panels-prop = 传播
panels-shade-map-where-signals-are = 按信号在每个频段上实际抵达的位置为地图着色。每一次接收都标在其路径的中点——即使之弯曲的那片电离层——而不是对方电台的位置，因此这张图反映的是天空，而非业余无线电爱好者所在的地方。它由本电台参与过的路径构成；若在“设置 → 信号”中开启了反向信标网络（RBN），还包括全球 skimmer 所听到的内容，这涵盖了此电台并未收听的频段。
panels-night = 夜间
panels-shade-where-sun-down-twilight = 在太阳落下的地方以及其间的暮光区域着色，使灰色分界线显示在地图上。在其夜侧，低频段延伸得更远，高频段则收缩；而分界线本身正是 DX 所在之处。
panels-all-bands = 所有频段
panels-every-band-once-one-hue = 所有频段同时显示，每个频段一种色调——总体条件。
panels-one-band = 单个频段
panels-one-band-cold-hot-blue = 单个频段，由冷到热：蓝、绿、黄、红。
panels-frequency-you-are-actually-working = 你实际工作的频率：表盘加上其旁边的音调偏移。这才是应记入日志的内容——仅表盘频率会低这么多。
panels-send-return = 返回时发送
panels-freq = ⇵ 频率
panels-nothing-agreed-mode-save-your = 此模式下没有约定的频率——请在下方保存你自己的频率。
panels-forget-saved-frequency = 删除此已保存的频率
panels-dial-already-one-your-saved = 此表盘频率已是你保存的频率之一。
panels-dial-already-agreed-list-mode = 此表盘频率已在此模式的约定列表中。
panels-decode-squelch-raise-stop-decoding = 解码静噪——调高以停止解码噪声
panels-message-repeats-long-transmit-held = 只要按住发射，消息就会重复——流星会捕捉到正在经过的那一部分。
panels-w1abc-w9xyz-fn42 = W1ABC W9XYZ FN42
panels-tx = {"  "}发射开{"  "}
panels-tx-2 = {"   "}发射{"   "}
panels-call-cq = {" "}呼叫 CQ{" "}
panels-nocall = 无呼号

## 发射音频窗口（src/app/tx_audio.rs）

txaudio-heard-too-little-speech-talk = 听到的话音太少——请在整个计数期间正常说话，然后重试。
txaudio-tx-audio = 发射音频
txaudio-mode-not-voice-mode-so = 此模式不是话音模式，因此它发送的任何内容都不经过此处理链。在此所做的设置对 SSB、AM 和 FM 生效。
txaudio-bypass = 旁路
txaudio-switch-every-stage-out-once = 一次性将每一级（包括降噪器）全部切出，使话筒以未处理的方式发出。再次点击可精确恢复原先开启的各级——这是对比处理链作用的最快方法。
txaudio-denoise = 降噪
txaudio-rnnoise-microphone-ahead-every-other = 对话筒使用 RNNoise，位于所有其他级之前，因此在噪声门或压缩器将室内噪声提升之前，它已被去除。会增加约 10 ms 的延迟。
txaudio-digital-voice = 数字话音
txaudio-also-run-freedv-rade-digital = 同时让 FreeDV / RADE 数字话音经过此处理链。默认关闭：声码器是在自然语音上训练的，对于经过大量处理的声音，其解码效果可能更差，因此在依赖它之前，请先在空中试用。TCI 音频和数字模式从不经过此处理链。
txaudio-test = 测试
txaudio-rec = {" "}⏺ 录制{" "}
txaudio-play = {" "}▶ 播放{" "}
txaudio-play-test-phrase-through-chain = 按当前设置，将测试短语经过处理链播放到你的扬声器。在播放时进行调整，即可听到变化。播放期间，接收音频会被替换。
txaudio-recording = 录制中……
txaudio-no-phrase-yet = 尚无短语
txaudio-live = 实时
txaudio-live-2 = {" "}🎧 实时{" "}
txaudio-hear-your-microphone-through-chain = 在你说话时，通过处理链听到自己的话筒，而无需按下发射机。请使用耳机：通过扬声器会产生反馈。你会听到自己晚几十毫秒的声音，这会使说话变得别扭——请用表头设置电平，并用“录制 / 播放”判断声音效果。运行期间，接收音频会被替换；按下 PTT 即可停止。
txaudio-meters = {" "}表头{" "}
txaudio-run-your-microphone-through-chain = 仅为了表头而让话筒经过处理链：扬声器上没有任何声音，因此没有需要与之对话的延迟，也不会产生反馈。这是设置话筒和输入增益、噪声门阈值以及压缩器工作强度的方法。接收音频继续；按下 PTT 即可停止。
txaudio-monitoring-live-use-headphones-speakers = 正在实时监听——请使用耳机，否则扬声器会反馈到话筒中。
txaudio-voice-modes-only-tci-clients = 仅限话音模式。TCI 客户端和数字模式会绕过此处理链；使用它时，请将电台自身的发射均衡器和处理器保持为平直。
txaudio-auto-set-stopped-microphone-was = 自动设置已停止：话筒已被占用。
txaudio-microphone-may-clipping-before-gets = {" "}话筒在到达此处之前可能已削波——若听起来刺耳，请调低它自身的增益。
txaudio-level = 电平
txaudio-mic-control-same-one-transmit = 话筒控件——与发射栏上的是同一个，由所有配置文件共用。中间为 0 dB，顶端为 +6 dB。
txaudio-mic = 话筒
txaudio-gain-top-mic-microphone-stored = 在话筒控件之上，针对此话筒的增益，与配置文件一同保存。预设的阈值假定话音在 IN 表头上的峰值约为 -10 dBFS；“自动设置”会将其设到此处。在“旁路”下仍然生效，并且也适用于数字话音：它是电平，而非处理。
txaudio-input = 输入
txaudio-auto-set = 自动设置
txaudio-auto-set-cancelled = 自动设置已取消。
txaudio-talk-now-you-would-air = 现在请像在空中那样说话……
txaudio-db = -∞ dB
txaudio-chain = 处理链
txaudio-order-audio-meets-them = 音频经过它们的顺序
txaudio-switch-stage = 开启此级
txaudio-earlier-chain = 处理链中的前面
txaudio-later-chain = 处理链中的后面
txaudio-output-limiter = 输出限幅器
txaudio-output = 输出
txaudio-trim-limiter-always-last = 修整和限幅器——始终位于最后
txaudio-profiles = 配置文件……
txaudio-gate-expander = 噪声门 / 扩展器
txaudio-pushes-room-noise-fan-rumble = 在停顿时把室内噪声、风扇隆隆声和嘶嘶声压低。将阈值设在底噪之上、最轻的音节之下。
txaudio-gate = 噪声门
txaudio-hard-gate-10-1-down = 硬噪声门：10:1，最多降至 -40 dB
txaudio-expander = 扩展器
txaudio-gentle-expander-2-1-never = 柔和扩展器：2:1，最多不超过 -15 dB——室内声音不会变得死寂
txaudio-reset = 复位
txaudio-back-defaults = 恢复默认值
txaudio-threshold = 阈值
txaudio-below-gate-starts-close = 低于此值时，噪声门开始关闭。
txaudio-return = 回差
txaudio-hysteresis-once-open-closes-only = 迟滞：一旦打开，它只在阈值以下这么多时才关闭，因此在阈值附近徘徊的话音不会使噪声门抖动。
txaudio-hold = 保持
txaudio-how-long-stays-open-after = 话音下降后，它会保持打开多久，使词尾不被切掉。
txaudio-release = 释放
txaudio-how-fast-closes-once-hold = 保持时间结束后，它关闭的速度。如果噪声门出现抽动，请加长此时间。
txaudio-floor = 下限
txaudio-most-ever-turns-pauses-down = 它在停顿时最多能将声音压低的量。
txaudio-ratio = 比率
txaudio-how-steeply-closes-below-threshold = 它在阈值以下关闭的陡峭程度。
txaudio-look-ahead = 预读
txaudio-opens-much-before-word-starts = 在一个词开始之前提前这么多打开，使其首个辅音不被切掉。会为所有发射内容增加相同的延迟。
txaudio-high-pass-low-shelf-two = 高通、低搁架、两个峰值、高搁架和低通。拖动曲线上的一个点，或在下方设置。将低通设为你的发射滤波器的宽度。
txaudio-flat = 平直
txaudio-every-band-back-0-db = 所有频段恢复到 0 dB，两端均关闭
txaudio-freq = 频率
txaudio-gain = 增益
txaudio-q-slope = Q / 斜率
txaudio-high-pass = 高通
txaudio-removes-handling-noise-breath-pops = 去除转角以下的手持噪声、呼吸爆破音和嗡嗡声。
txaudio-low-shelf = 低搁架
txaudio-body-mud-cut-little-here = 厚度，即浑浊感：若话音听起来像关在箱子里，请在此处稍微削减。
txaudio-peak-1 = 峰值 1
txaudio-bump-dip-voice-band-often = 话音频段中的凸起或凹陷——通常是 300–500 Hz 附近的削减。
txaudio-peak-2 = 峰值 2
txaudio-presence-lift-around-2-3 = 临场感：2–3 kHz 附近的提升，是 SSB 上穿透力的来源。
txaudio-high-shelf = 高搁架
txaudio-brightness = 明亮度。
txaudio-low-pass = 低通
txaudio-stops-energy-above-transmit-filter = 防止超出发射滤波器的能量被浪费在调制器中。
txaudio-t-24-db-oct = 24 dB/倍频程
txaudio-t-12-db-oct = 12 dB/倍频程
txaudio-click-steeper-gentler-slope = 点击以切换更陡或更缓的斜率
txaudio-de-esser = 去齿音器
txaudio-turns-down-only-sibilant-band = 仅当“s”和“sh”音超过阈值时才压低齿音频段，因此话音的其余部分保持其电平。可在说“sister silver cellophane”的同时扫描频率，并将其停在嘶嘶声最响之处来找到该频率。
txaudio-frequency = 频率
txaudio-centre-sibilant-band = 齿音频段的中心。
txaudio-width-band-higher-narrower = 频段的宽度——越高则越窄。
txaudio-band-level-where-reduction-starts = 频段电平，超过此值时开始压低。过低则会一直在工作，使话音变得发闷。
txaudio-amount = 量
txaudio-most-band-ever-turned-down = 频段最多被压低的量。从 -6 dB 左右开始。
txaudio-attack = 起振
txaudio-how-fast-reacts = 它反应的速度。
txaudio-how-fast-lets-go = 它松开的速度。
txaudio-compressor = 压缩器
txaudio-brings-quiet-syllables-up-holds = 将轻柔的音节提起，并压制响亮的音节。峰值处的压缩量以 3–6 dB 为目标；超过 10 dB 则是明显的压缩，更适合比赛。
txaudio-level-above-which-starts-compress = 超过此电平即开始压缩。
txaudio-how-hard-compresses-above-threshold = 超过阈值后压缩的力度。
txaudio-how-fast-reacts-peak-10 = 它对峰值反应的速度。10–25 ms 可使辅音保持清晰。
txaudio-how-fast-recovers = 它恢复的速度。
txaudio-knee = 拐点
txaudio-soft-knee-width-0-hard = 软拐点宽度；0 为硬拐点。
txaudio-makeup = 补偿
txaudio-gain-after-compression-bring-level = 压缩之后的增益，用于将电平重新提起。
txaudio-drive = 驱动
txaudio-gain-ahead-compressor-so-more = 压缩器之前的增益，使更多的话音越过阈值——提高平均功率。应与输出限幅器配合使用。
txaudio-phase = 相位
txaudio-all-pass-phase-rotator-ahead = 检波器之前的全通相位旋转器。它可以抚平话音中不对称的峰值，使相同的峰值电平携带更多的平均功率。4 是常见的广播设置；0 为关闭。
txaudio-last-stage-trim-look-ahead = 最后一级：修整，以及一个预读限幅器，它把每个峰值都限制在上限之下，而不会产生硬削波所造成的频谱扩散。默认关闭，因此开启处理链不会改变你已经设定的驱动电平。
txaudio-limiter = 限幅器
txaudio-on = 开
txaudio-ceiling = 上限
txaudio-no-peak-leaves-chain-above = 没有任何峰值会以高于此值的电平离开处理链。
txaudio-trim = 修整
txaudio-output-gain-ahead-limiter = 输出增益，位于限幅器之前。
txaudio-off = {"  "}关
txaudio-meters-move-while-you-transmit = 当你发射话音、播放测试短语或运行“实时”或“表头”时，表头会移动。
txaudio-profiles-2 = 配置文件
txaudio-profile-whole-chain-every-stage = 配置文件即整个处理链：每一级的设置、哪些级已开启、它们的顺序，以及你的话筒的输入增益。加载一个配置会替换整条处理链；话筒控件和驱动保持不变。
txaudio-name = 名称
txaudio-save = 保存
txaudio-store-chain-now-under-name = 以此名称保存当前的处理链，并替换同名的配置文件
txaudio-import = 导入……
txaudio-read-profile-json-file = 从 .json 文件读取配置文件
txaudio-tx-audio-profile = 发射音频配置文件
txaudio-no-profiles-yet-save-stores = 尚无配置文件。“保存”会存储你现在设置好的处理链。
txaudio-load = 加载
txaudio-export = 导出……
txaudio-delete = 删除
txaudio-starting-points = 起始点
txaudio-set-your-level-auto-set = 请先用“自动设置”设定你的电平——预设的阈值以此为前提——然后从此处进行调整，而不要按加载时的设置直接发射：录制一段短语，并在每次更改后收听。预设会保留你的输入增益。
txaudio-profile-loaded = 配置文件已加载。

## 太阳系三维视图叠加层（src/solar3d/overlay.rs）

solar-prop = 传播
solar-where-signals-are-actually-getting =
    信号在每个频段上实际抵达的位置。本电台参与的每一次接收——WSPR、FT8/FT4、JS8 以及日志——都标在其路径的中点，即使之弯曲的那片电离层，而不是对方电台的位置。因此这张图反映的是天空，而非业余无线电爱好者所在的地方。

    它只能显示本电台作为其一端的路径。海洋之所以亮起，是因为长路径的中点落在那里；南极之所以保持黑暗，是因为没有人从那里发射。默认关闭——它会绘制整个星球。
solar-orbits = 轨道
solar-orbital-paths = 轨道路径
solar-clouds = 云层
solar-cloud-cover-noaa-s-global = 云层覆盖来自 NOAA 的全球静止卫星拼图，以一层空气的厚度绘制，而不是贴在表面上的图片：红外数据表明每个云顶有多冷，由此决定它有多高。雷暴会在它们所形成的塔状结构内闪烁。
solar-planets = 行星
solar-other-seven-planets-eighteen-their = 其余七颗行星、它们的十八颗卫星，以及土星和天王星的环。行星位置来自 JPL 的开普勒轨道根数集；卫星则是拟合到 JPL Horizons 的圆形轨道。
solar-coronal-mass-ejection-trajectory-cones = 日冕物质抛射轨迹锥
solar-sun-obs = 太阳观测
solar-solar-observations-sun-s-disk = 太阳圆面上的太阳观测：太阳黑子活动区，以及耀斑的来源
solar-labels = 标签
solar-body-region-labels = 天体和区域标签
solar-small-bodies = 小天体
solar-names-asteroids-comets-they-are = 为小行星和彗星标注名称。无论如何它们都会被绘制——这里只决定是否同时绘制三十五个编号。无论此项如何设置，相机所对准的天体以及查找框匹配的任何东西，都会保持标注名称。
solar-decoded-ft8-ft4-stations-path = 已解码的 FT8/FT4 电台，以及通往所通联电台的路径
solar-sats = 卫星
solar-amateur-radio-satellites-their-orbits = 业余无线电卫星、它们的轨道，以及从你的 QTH 看到的仰角
solar-aurora = 极光
solar-auroral-oval-noaa-s-ovation = 来自 NOAA OVATION 模型的极光椭圆，以其真实高度绘制为发光壳层，并在表面上标出 10% 等值线的赤道侧边缘
solar-awards = 奖项
solar-award-coverage-every-dxcc-entity = 奖项覆盖：每个 DXCC 实体都按你的日志仍然缺少的内容着色。橙色表示你从未与该实体通联过，琥珀色表示你通联过但未获确认，暗绿色表示已收到 QSL 卡。跟随“奖项”窗口中的频段过滤器。
solar-view = 视图
solar-which-body-camera-orbits-animated = 相机绕行哪个天体，以及动画导览
solar-layers = 图层
solar-what-drawn-orbits-clouds-cmes = 绘制的内容：轨道、云层、日冕物质抛射、标签、通联、极光
solar-which-sdo-channel-wraps-sun = 哪个 SDO 通道环绕太阳，以及它有多旧
solar-weather = 天气
solar-aurora-propagation-numbers-which-bands = 极光、传播数值，以及哪些频段开放
solar-space-weather = 空间天气
solar-nothing-measured-yet = 尚无测量数据
solar-scale = 比例
solar-body-moon-orbit-exaggeration = 天体和月球轨道的夸大
solar-time = 时间
solar-scrub-whole-scene-forward-back = 前后拖动整个场景
solar-activity = 活动
solar-replay-last-hour-decodes-globe = 在地球上回放过去一小时的解码
solar-what-propagation-heat-map-showing = 传播热力图显示的内容
solar-propagation = 传播
solar-auto = ▶ 自动
solar-fly-spline-through-set-framed = 沿一组取景视点飞行样条曲线。任何鼠标操作都会将其取消。
solar-moons-ride-circular-orbits-fitted = 卫星沿拟合到 JPL Horizons 的圆形轨道运行：与它们真实位置相差一到两度以内，而对于米兰达（Miranda）则最多相差六度，因为其轨道平面摆动太快，圆形无法跟随。
solar-fetch-everything-again-now = 立即重新获取所有内容
solar-all-sats = 所有卫星
solar-show-every-satellite-element-set = 显示轨道根数集中的每一颗卫星，而不仅是热门的几颗。轨道环仍只保留精选的少数——同时显示九十个会无法辨认。
solar-starting = 正在启动……
solar-body = 天体{" "}
solar-moon-orbit = 月球轨道{" "}
solar-stretch-earth-moon-distance-so = 拉伸地球到月球的距离，使这一对天体能够被分开看到
solar-sun = 太阳{" "}
solar-sun-radius-exaggeration-leave-1 = 太阳半径的夸大。保持为 1× 以使日冕物质抛射的几何关系清晰可读——膨胀的太阳会吞没每个锥体的底部。
solar-show = 显示
solar-paint-globe-what-getting-through = 按信号抵达情况为地球着色
solar-paints-globe-what-getting-through = 在每个频段上，按信号实际抵达的情况为地球着色。每一次接收都标在其路径的中点——即电离层发挥作用之处——而不是对方电台的位置。它由本电台所听到的内容构成，若开启了反向信标网络，还包括全球 skimmer 的数据。
solar-display = 显示
solar-all-bands = 所有频段
solar-every-band-once-one-hue = 所有频段同时显示，每个频段一种色调，并在两者重叠之处混合颜色。这是回答“条件如何”的视图，而不是“二十米开放了吗”。
solar-one-band = 单个频段
solar-one-band-cold-hot-blue = 单个频段，由冷到热：蓝、绿、黄、红。
solar-band = 频段
solar-nothing-heard-yet = 尚未听到任何内容
solar-bands = 频段
solar-sources = 来源
solar-contacts-logbook-path-was-open = 日志中的通联。路径是开放的，但没有值得一提的信号报告——RST 不是 SNR，因此这些通联只计入某个小区的繁忙程度，从不计入其余量。
solar-stations-reported-hearing-one-downloaded = 报告收听到本电台的电台，从 WSPRnet 下载。这是发射信标唯一能得到的反馈。
solar-reverse-beacon-network-skimmers-what = 反向信标网络的 skimmer：每个频段上其他所有人正在听到的内容，包括本电台并未收听的频段。它比其他数据粗略——RBN 的记录不带网格定位，因此双方都被放在其所在国家的中心，对小国而言相当接近，对大国则偏差很大。在“设置 → 信号”下开启。
solar-decodes-station-made-each-measured = 本电台做出的解码。每一项都按其自身模式的底噪进行测量，因此 WSPR 报告与 FT4 报告可以相互比较。
solar-memory = 记忆
solar-half-life = 半衰期{" "}
solar-how-long-reception-takes-count = 一次接收需要多长时间，其计入的权重才会减半。电离层自身的记忆很短——两小时前的开放不应与两分钟前的开放争论不休。
solar-live = 实时
solar-follow-band-happens = 随频段的变化实时跟随
solar-replay = ⏸ 回放
solar-replay-2 = ▶ 回放
solar-replay-last-hour-decodes-over = 反复回放过去一小时的解码
solar-min-ago = {" "}分钟前
solar-where-last-hour-globe-showing = 地球当前显示的是过去一小时中的哪个时刻
solar-trail = 轨迹{" "}
solar-how-long-decode-s-arc = 一次解码的弧线在磁头身后停留在地球上的时长
solar-speed = 速度{" "}
solar-how-much-faster-than-real = 回放比实时快多少
solar-qso-layer-off = 通联图层已关闭
solar-no-decodes-yet = 尚无解码
solar-qth-not-set-enter-your = 未设置 QTH——请在“设置”中输入你的网格定位
solar-mode = 模式
solar-path = 路径
solar-sent = 发送
solar-rcvd = 接收
solar-grid = 网格
solar-dxcc = DXCC
solar-geostationary-always-above-your-horizon = 静止轨道——始终位于你的地平线之上。
solar-no-passes-next-48-hours = 从你的 QTH 看，未来 48 小时内没有过境。
solar-start = 开始
solar-max-el = 最大仰角
solar-aos-los-are-azimuths-horizon = AOS/LOS 是地平线上的方位角。时间为 UTC。
solar-locked-unlock = {" "}● 已锁定 — 解锁{" "}
solar-release-satellite-lock = 释放卫星锁定
solar-lock = {" "}锁定{" "}
solar-track-satellite-main-window-doppler = 在主窗口中跟踪此卫星：经多普勒校正的收发、按转发器映射的上行，以及旋转器转向
solar-no-frequencies-file-one-add = 此卫星没有已存档的频率——请在“设置 ▸ TLE”中添加。
solar-frequencies-yours = 频率 · 你的
solar-frequencies = 频率
solar-link = 链路
solar-downlink-mhz = 下行（MHz）
solar-uplink-mhz = 上行（MHz）
solar-doppler-shifts-these-few-khz = 在一次 LEO 过境期间，多普勒频移会使它们偏移几 kHz。
solar-sim = -- 模拟
solar-satellite-planet-comet = 卫星、行星或彗星
solar-planet-asteroid-comet = 行星、小行星或彗星
solar-clear-search = 清除搜索
solar-no-match = 无匹配
solar-no-sounder = 无探测器数据
solar-heard = 收听 ≥
solar-kp = Kp / A
solar-x-ray = X 射线
solar-observed-above-sounder-better-than = 探测器观测值之上——优于模型
solar-power-n-s = 功率 N/S
solar-edge-n-s = 边缘 N/S
solar-kp-peak-24-h = 24 小时 Kp 峰值
solar-observed-forecast = 观测 | 预报
solar-bands-open = 开放的频段
solar-dxcc-coverage = DXCC 覆盖
solar-ir-vis = 红外+可见光
solar-ir-only = 仅红外
solar-no-deep-convection = 无深对流
solar-t-1-storm = 1 次风暴
solar-glancing = {" "}· 擦过
solar-direction-estimated = {" "}· 方向为估算值

## 数字模式设置对话框（src/app/panels/setup.rs）

setup-aprs-setup = APRS 设置
setup-packet-setup = 分组设置
setup-ft8-ft4-ft2-setup = FT8 / FT4 / FT2 设置
setup-my-callsign = 我的呼号
setup-my-grid = 我的网格
setup-station-call = 电台呼号
setup-ssid-oe3jjs-10-packet-station = 带 SSID——OE3JJS-10。分组电台通常使用与操作员不同的 SSID，而且在此设置之前不会发射任何内容。
setup-speed = 速率
setup-t-9600-needs-radio-s-data = 9600 需要电台的数据端口。麦克风和扬声器通路会在两端破坏它——这是电台的问题，而非 sdroxide 的。
setup-tx-delay = 发射延迟
setup-flags-sent-before-frame-so = 在一帧之前发送标志，使对方能够听到我们并锁定其时钟。在 CAT 电台上，仅 sdroxide 自身就要花 165–240 ms 才能发射，电台自身的发射就绪时间还要再加上去。
setup-packet-length = 分组长度
setup-bytes = {" "}字节
setup-most-single-frame-carries-shorter = 单帧所能承载的最大长度。较短的帧能在勉强可用的路径上存活——只需重发被击中的那一帧——但开销更大。HF 上为 128，路径可靠时为 256。
setup-frames-sent-before-waiting-acknowledgement = 等待确认之前发送的帧数。越多越能充分利用良好的路径；在糟糕的路径上，则意味着更多需要重发的内容。
setup-extended = 扩展
setup-ask-mod-128 = 请求 mod-128
setup-extended-sequence-numbers-window-bigger = 扩展序列号，用于窗口大于七的情况。许多节点会以 DM 拒绝该请求，这看起来与一个不愿与你通话的电台完全相同——因此除非你确知对方需要它，否则请保持关闭。
setup-answer-calls = 应答呼叫
setup-accept-connections = 接受连接
setup-off-winlink-client-which-dials = 对于 Winlink 客户端应保持关闭，因为它是拨出的，没有理由应答。开启后则可作为对等方被访问：呼叫会出现在终端面板中，你即与发起呼叫的一方通话。若某个电台的链路已在进行 Winlink 会话，它会在会话结束前拒绝呼叫。
setup-connect-text = 连接文本
setup-what-caller-greeted = 呼叫者所收到的问候语
setup-sent-station-connects-us-once = 链路建立后，发送给连接到我们的电台。留空则什么也不发送——这会让呼叫者以为出了故障。
setup-default-via = 默认经由
setup-oe3xlr-1-oe3xms-1 = OE3XLR-1,OE3XMS-1
setup-digipeater-path-terminal-starts-path = 终端开始时使用的数字中继路径。通往你本地节点的路径每次都相同，而重新输入它正是某一跳被遗漏的原因。
setup-beacon = 信标
setup-t-0-disables-timer = 0 表示禁用计时器
setup-beacon-text = 信标文本
setup-kiss-server = KISS 服务器
setup-serve = 服务
setup-offer-modem-kiss-tnc-socket = 将此调制解调器作为 KISS TNC 通过套接字提供，使 Pat、APRS 客户端或 Linux AX.25 协议栈都能使用电台。
setup-aprs-call = APRS 呼号
setup-callsign-station-beacons-under-ssid = 此电台以其发射信标所用的呼号，连同其 SSID——汽车用 OE3JJS-9，I-gate 用 -10，手机用 -5。留空则使用上面的呼号。
setup-no-callsign-station-will-not = 没有呼号：此电台将完全不发射——既不发射信标，也不发送消息，也不发送确认。
setup-symbol = 符号
setup-two-characters-protocol-carries-them = 协议所携带的两个字符：一个表（`/` 或 `\`）和一个符号代码。第一位的数字或字母是叠加层，会绘制在备用表图标之上。
setup-path = 路径
setup-wide1-1-wide2-1 = WIDE1-1,WIDE2-1
setup-how-far-you-ask-repeated = 你要求被中继的距离。这是该信道上影响最大的单一设置：每一跳都会使整个网络在同一个共享频率上的发射次数成倍增加。
setup-position = 位置
setup-my-grid-2 = 来自我的网格
setup-beacon-centre-locator-above-reported = 以上面网格定位的中心为信标位置，并附上网格定位实际具有的模糊度来报告——六位字符的网格定位跨度约为两公里，如实说明这一点是诚实的做法。取消勾选则改为给出精确坐标。
setup-latitude = 纬度
setup-longitude = 经度
setup-locator = 来自网格定位
setup-not-coordinate = 不是坐标
setup-comment = 备注
setup-sent-every-beacon = 每次信标都会一同发送
setup-t-0-default-never-beacons-timer = 0——默认值——从不按计时器发送信标。三十分钟是固定电台的惯例；移动电台发送得更频繁，但每一次信标都占用别人的信道时间。
setup-compressed = 压缩
setup-compressed-position-format-third-air = 压缩位置格式：空中时间只需三分之一，而且更精确。自 1990 年代以来的每一台接收机都能读取它。
setup-first-goes-out-one-interval = 第一次信标会在从现在起一个间隔后发出，而不是立即发出。
setup-messages = 消息
setup-acknowledge = 确认
setup-answer-messages-addressed-you-acknowledgement = 应答发给你的消息。确认是本电台在无人请求时发出的发射，因此仅接收的设置应将其关闭——这样信标也就不再声称可以被联系到。
setup-flags-sent-ahead-every-frame = 在每一帧之前发送的标志，使对方的调制解调器能在数据开始之前听到载波并锁定其时钟。它必须长于从按下发射到辐射出去之间的全部时间：仅 sdroxide 自身在 CAT 电台上就要占用其中的 165–240 ms，而通过网络接收音频的电台还会在此之上再加缓冲。设得太短，对方永远无法锁定——发射确实存在，却没有任何东西能将其解码。
setup-shared-packet-mode = 与分组模式共用
setup-keep-stations = 保留电台
setup-how-long-station-stays-map = 电台最后一次被听到之后，会在地图上保留多久。这也是地图淡出所依据的时间，因此较短的窗口只显示当前活跃的内容。
setup-auto-reply = 自动应答
setup-answer-snr-grid-status = 应答 SNR? / GRID? / STATUS?
setup-answer-direct-question-addressed-you = 应答发给你或发给 @ALLCALL 的直接问题，以答复代替确认。从不应答其他电台的通信，也从不应答它自己。
setup-heartbeat = 心跳
setup-off = 关
setup-turbo-does-not-beacon-local = Turbo 不发送信标——它是本地和 VHF 的速率。
setup-sends-your-callsign-grid-so = 发送你的呼号和网格，使他人知道你可以被接收。第一次会在从现在起一个间隔后发出，而不是立即发出。
setup-beacon-frequency = 信标频率
setup-t-500-1000-hz = 500–1000 Hz
setup-move-each-beacon-free-slot = 与 JS8Call 的做法一样，将每个信标移至心跳子频段中的空闲时隙：监听信标的电台正是在那里寻找它，而且这能使无人值守的发射机不干扰别人的通联。时隙在信标真正发出时选定，并避开所有正在解码的内容。
setup-working-freq = 工作频率
setup-beacon-where-you-are-working = 改为在你正在工作的频率上发送信标。这与频段惯例相悖，但能使你所发射的一切都集中在一处。
setup-heartbeat-reply = 心跳应答
setup-answer-heartbeats-signal-report = 以信号报告应答心跳
setup-tell-station-beaconed-how-well = 告诉发送信标的电台你接收它的效果如何。默认关闭：繁忙的频段每个时隙都会有心跳，而全部应答会淹没正是心跳所用来保持安静的频段。每个电台每 15 分钟只应答一次，并且在仍有消息到达时，或你有待发送的内容时，绝不应答。
setup-status-message = 状态消息
setup-tx-period = 发射时段
setup-even = 偶数
setup-odd = 奇数
setup-auto-sequence = 自动序列
setup-decode-depth = 解码深度
setup-fast = 快速
setup-normal = 标准
setup-deep = 深度
setup-how-hard-ft8-decoder-works =
    FT8 解码器对弱信号的工作力度。

    快速——单次通过，不做信号减除：最快，解码数最少。
    标准——均匀的多次通过减除：比深度略快，彻底程度略低。
    深度——带检查点的多次通过：解码数最多，在繁忙的时隙上约需 1.2 秒。
setup-auto-tx-frequency = 自动发射频率
setup-choose-transmit-frequency-automatically-quietest = 自动选择发射频率：在你发射的时段中最安静的位置，而不是你所应答的电台的频率。关闭时并不会保持该频率——它会在被呼叫电台的频率上应答。若要保持，请使用“保持发射频率”。
setup-hold-tx-frequency = 保持发射频率
setup-pin-transmit-tone-where-nothing = 将发射音调固定在当前位置。没有任何东西会移动它：应答电台、呼叫队列、呼叫 CQ、点击解码或瀑布图都不会。它会覆盖“自动发射频率”。更换频段是唯一的例外：你在新频段上最后设定的偏移会随之恢复。若你的执照比频段计划更窄——例如在英国 60 米的 5357 kHz 表盘上，分配在 5358.0 结束，因此音调必须保持在 1000 Hz 以下。
setup-tx-watchdog = 发射看门狗
setup-stop-transmitting-after-long-no = 在这么长时间没有回复且你也没有任何操作之后，停止发射。0 表示禁用。
setup-give-up-after = 在以下次数后放弃
setup-calls = {" "}次呼叫
setup-unanswered-calls-one-station-before = 在转向下一个电台之前，对同一电台未获应答的呼叫次数。呼叫 CQ 不受此限制。0 表示禁用。
setup-contest = 比赛
setup-everyday-exchange-grid-report-rr73 = 日常交换：网格、报告、RR73。
setup-european-vhf-contests-send-signal = 欧洲 VHF 比赛：发送信号报告、序列号以及你的六位网格定位，采用 WSJT-X 称为 EU VHF Contest 的消息格式。CQ 发送为 CQ TEST。
setup-serial = 序列号
setup-serial-number-next-exchange-will = 下一次交换将携带的序列号。它会随着每次通联被记录而自行递增，并在会话之间保留——只有在开始一场比赛，或要从纸质日志停下的地方接着记录时，才需手动设置。
setup-needs-6-character-grid = ⚠ 需要六位网格定位
setup-eu-vhf-contest-messages-carry = 欧洲 VHF 比赛消息携带的是六位网格定位（JN88DD，而不是 JN88）。在“我的网格”中设置好之前，将改为发送普通的交换内容。
setup-dxpedition = DXpedition
setup-ordinary-ft8-operation = 普通 FT8 操作。
setup-calling-dxpedition-running-fox-mode = 呼叫运行 Fox 模式的 DXpedition：在 1000 Hz 以上呼叫，当它应答时移至 Fox 的频率，并在它的 RR73 上记录通联，而不发送 73。
setup-run-pile-up-several-signals = 运行大批呼叫：同时发送多个信号，按队列接收呼叫者，最强和最稀有的优先通联。“呼叫 CQ”开始它，“停止通联”则使其停止。
setup-fox-signals = Fox 信号
setup-once = {" "}同时
setup-simultaneous-transmissions-spaced-60-hz = 同时发射，彼此相隔 60 Hz。它们共用发射机的功率，因此信号越多，每个信号就越弱。
setup-tx-audio = 发射音频
setup-how-loud-mode-s-burst =
    此模式的脉冲以多大音量交给自行调制它的电台——在 FM 上，这就是频偏，因为 FM 发射机会把音频电平转换为频率摆幅，而且没有 ALC 来制约它。1200 波特分组大约需要 3 kHz 的频偏，而话音则需要 5 kHz，因此把满量程送入一个设为话音的数据输入，就会造成过度频偏——在收听者听来完全正常，却不能为任何人解码。请将其调低，直到其他电台报告能收到你，或在电台上设置电平。

    此电平仅属于此模式。你从未设置过的模式会从 FM 的数值开始，而边带模式则从它们自己的数值开始——因此为 1200 波特设置的频偏绝不会用到 FT8 上。
setup-how-loud-mode-s-over =
    此模式的发射以多大音量交给自行调制它的电台——例如声卡上的 CAT 电台、FLEX，或网络端口上的 Icom。在边带上，这是送入调制器的驱动：将其调低，直到电台的 ALC 几乎不动，然后在电台上设置功率，因为恒包络模式下 ALC 过度工作，正是造成频谱扩散的原因。这里驱动作用于电台的功率寄存器，而不是其音频，因此这才是电平。

    此电平仅属于此模式：FT8、RTTY、PSK 和 MCW 各自保存自己的设置，因为它们加载调制器的方式不同。同一个滑轨也位于发射栏上，你在发射时可以在那里调整它。
setup-deviation-fm = 频偏，FM
setup-drive-ssb = 驱动，SSB
setup-grid = 网格
setup-report = 报告
setup-r-report = R+报告

## 功放窗口（src/app/amp.rs）

amp-t-10-w-scale-rescaled = ，10 W 刻度，已重新缩放
amp-t-10-w-scale = ，10 W 刻度
amp-t-2-w-scale-rescaled = ，2 W 刻度，已重新缩放
amp-t-2-w-scale = ，2 W 刻度
amp-t-100-w-scale-rescaled = ，100 W 刻度，已重新缩放
amp-t-100-w-scale = ，100 W 刻度
amp-t-20-w-scale-rescaled = ，20 W 刻度，已重新缩放
amp-t-20-w-scale = ，20 W 刻度
amp-rescaled = ，已重新缩放
amp-and = {" "}与{" "}
amp-not-while-transmitting-switching-relay = 发射期间不可用：在有射频的情况下切换继电器可能会损坏它。请先松开发射。
amp-no-amplifier-set-up-choose = 尚未设置功放。请在“设置 → 功放”中选择其端口。
amp-settings = 设置
amp-dock = 停靠
amp-put-amp-window-back-inside = 将功放窗口放回主窗口内部
amp-pop-out = 弹出
amp-give-amp-window-window-own = 为功放窗口提供其自己的窗口，以便移至另一块屏幕
amp-rig = ，来自电台
amp-rig-flagged-reading = ，电台标记了此读数
amp-drv = ，来自 DRV
amp-rig-s-swr = ，来自电台的驻波比
amp-ptt-keys-amplifier-radio-off = PTT 随电台一起按下功放；OFF 则使其保持旁路。点击即可切换。
amp-switch-antenna-jack = 切换天线插孔
amp-band-down = 降低频段
amp-band-up = 提高频段
amp-tuner = 调谐器
amp-tuned = 已调谐
amp-tune = 调谐
amp-start-tune-then-key-steady = 开始调谐——然后按下稳定的载波（CW、FM 或 RTTY）。再次按下即停止。
amp-put-tuner-line-bypass = 将调谐器接入线路，或将其旁路
amp-tuning = 正在调谐……
amp-bottom = 底部
amp-pick-any-reading-either-needle = 为任一指针选择任意读数。上指针为白色，下指针为蓝色。
amp-forward-power-measured-radio-amplifier = 正向功率，由电台测量：功放在 OFF 状态下不报告此项
amp-forward-power-taken-drv-amplifier = 正向功率，取自 DRV：功放在 OFF 状态下不报告此项，而在旁路状态下，电台的功率直接通过
amp-forward-power = 正向功率
amp-reflected-power-measured-radio-amplifier = 反射功率，由电台测量：功放在 OFF 状态下不报告此项
amp-reflected-power-worked-out-fwd = 反射功率，根据 FWD 和驻波比计算：功放在 OFF 状态下不报告此项
amp-reflected-power = 反射功率
amp-standing-wave-ratio-red-past = 驻波比：达到或超过调谐上限（“设置 → 常规”）时显示为红色。请检查天线
amp-standing-wave-ratio-measured-radio = 驻波比，由电台测量：功放在 OFF 状态下不报告此项
amp-standing-wave-ratio = 驻波比
amp-drive-radio = 来自电台的驱动
amp-drain-supply = 漏极电源
amp-drain-current = 漏极电流
amp-temp = 温度
amp-heat-spreader-temperature-yellow-49 = 散热片温度：49 °C（120 °F）起为黄色，超过 65 °C（149 °F）为红色，高于 70 °C（158 °F）则脱离线路。点击可切换 °C/°F。
amp-c = °C
amp-f = °F
amp-performance = 性能
amp-numbers-hold-peaks-bars-are = 数字保持峰值 · 条形为实时
amp-input-power = 输入功率
amp-supply-volts-drain-amps = 电源电压 × 漏极电流
amp-efficiency = 效率
amp-output-power-input-power = 输出功率 ÷ 输入功率
amp-gain = 增益
amp-output-power-over-drive-decibels = 输出功率相对于驱动的增益，单位为分贝
amp-off-line-over-temperature = 已脱离线路 · 温度过高
amp-connected-transmitting = 已连接 · 发射中
amp-connected = 已连接
amp-connecting = 正在连接……
amp-atu-bypassed = ATU 已旁路
amp-no-inductance-needed = 无需电感
amp-no-capacitance-needed = 无需电容
amp-c-out-antenna-side = C OUT · 天线侧
amp-c-transmitter-side = C IN · 发射机侧

## SSTV 面板（src/app/panels/sstv.rs）

sstv-chunks-received-lit-still-missing = 已接收的数据块（亮）和仍缺少的数据块（暗）
sstv-sstv = SSTV
sstv-auto = 自动
sstv-signal = 信号
sstv-station-sending-mode-sdroxide-does = 有电台正在发送一种 sdroxide 不支持的模式。信号和调谐都正常——只是没有针对此模式的解码器。你这一端无需做任何处理。
sstv-listening = 正在收听……
sstv-restart-rx = 重新开始接收
sstv-abandon-picture-being-received-listen = 放弃正在接收的图片，并收听下一个帧头。适用于以错误模式开始解码的发射——否则接收机会一直锁定在该模式上，直到它结束。
sstv-callsign-last-station-sent-fsk = 上一个电台在图片之后以 FSK ID 发送的呼号。
sstv-tx-slant = 发射倾斜
sstv-transmit-clock-trim-ppm-remove = 发射时钟微调（ppm），用于消除对方解码器上的倾斜
sstv-reset-0-ppm = 重置为 0 ppm
sstv-fsk-id = FSK ID
sstv-send-your-callsign-tones-after = 在每张图片之后以音调发送你的呼号——即 SSTV 中继台和其他程序读取的识别信息。会增加约 2.5 秒，并且在你设置呼号之前什么也不发送。
sstv-tx-lead = 发射提前量
sstv-silence-sent-after-keying-before = 按下发射之后、图片的引导信号和 VIS 码之前所发送的静音。若解码器漏掉此帧头中的任何部分，就根本显示不出图片，因此它覆盖了向电台请求 PTT 与电台真正发射之间的间隔。对于即时键控的 SDR，设为 0。
sstv-live = 实时
sstv-waiting-picture = 等待图片……
sstv-waiting-signal = 等待信号……
sstv-no-low-audio = 无音频 / 音频过低
sstv-received = 已接收
sstv-reading-radio-s-pictures = 正在读取电台的图片……
sstv-decoded-pictures-collect-here = 解码的图片会收集在这里。
sstv-click-enlarge-right-click-delete = 点击放大 · 右键删除
sstv-delete-picture = 删除此图片
sstv-loading = 加载中……
sstv-transmit = 发射
sstv-click-edit-slot-s-message = 点击编辑此时隙的消息 · 双击加载图片
sstv-change-image = 更换图片……
sstv-load-image = 加载图片……
sstv-clear = 清除
sstv-empty-slot-s-picture-message = 清空此时隙的图片（消息保留）
sstv-banner = 横幅……
sstv-what-printed-across-top-every = 本电台发送的每张图片顶部所印的内容
sstv-fetching-slot-s-picture = 正在获取此时隙的图片……
sstv-load-image-into-slot = 将图片加载到此时隙 →
sstv-drawn-slot-s-image = 绘制在此时隙的图片上
sstv-tx = {"   "}发射{"   "}
sstv-abort-tx = {" "}中止发射{" "}
sstv-delete-picture-store = 从存储中删除此图片
sstv-received-image = 接收的图片
sstv-no-longer-store = 已不在存储中
sstv-loading-full-size = 正在加载原始尺寸……
sstv-save-picture = 保存图片……
sstv-save-copy-computer = 在本计算机上保存一份副本
sstv-delete-sure = 删除——确定吗？
sstv-click-again-delete-good = 再次点击即永久删除
sstv-delete = 删除……
sstv-image-banner = 图片横幅
sstv-drawn-into-every-picture-station = 绘制在本电台发送的每张图片中，位于此时隙自身消息之上。
sstv-draw-banner = 绘制横幅
sstv-substituted-when-picture-composed = 图片合成时替换：
sstv-top-left = 左上
sstv-top-right = 右上
sstv-colours = 颜色
sstv-strip-top-edge-fades-black = 条带，在其顶边——它渐变为黑色，或渐变为其下方的渐变色
sstv-both-texts = 两段文字
sstv-gradient = 渐变
sstv-fade = 渐变至
sstv-fade-strip-top-colour-second = 将条带从其顶部颜色渐变为第二种颜色，而不是渐变为黑色。
sstv-text-outline = 文字描边
sstv-draw-outline-around-banner-text = 为横幅文字绘制描边，使其在繁杂或浅色的条带上仍然清晰可读。
sstv-text-gradient = 文字渐变
sstv-fade-banner-text-colour-second = 在图片宽度范围内，将横幅文字从其颜色渐变为第二种颜色。
sstv-rainbow-text = 彩虹文字
sstv-all-text-overriding-colours-above = 所有文字，覆盖上面的颜色
sstv-draw-every-text-picture-banner = 将图片中的每段文字——横幅和消息——绘制为水平彩虹色，忽略上面的颜色。
sstv-message = 消息
sstv-slot-message-s-text-colour = 时隙消息的文字颜色
sstv-outline = 描边颜色
sstv-height = 高度
sstv-how-tall-strip-pixels-transmitted = 条带的高度，以所发射图片的像素计。文字的大小由它决定——SSTV 帧只有 320 像素宽，因此更高的横幅才能使对方读清楚。
sstv-reset = 重置
sstv-back-callsign-left-version-right = 恢复为左侧的呼号和右侧的版本
sstv-waiting-station-s-digital-mode = 正在等待电台的数字模式设置加载……
sstv-rifp = RIFP
sstv-size = 大小
sstv-calling-frequency-draft-names = 草稿中所指定的呼叫频率
sstv-encode = 编码
sstv-how-picture-encoded-into-object = 图片如何编码进 RIFP 所携带的对象中。“自动”会逐一尝试，并发送最小的一个。
sstv-try-every-encoding-send-smallest = 尝试每一种编码，发送最小的一个（从不有损）
sstv-gray = 灰度
sstv-grayscale-depth-rifp-s-raster = 灰度位深。按定义，RIFP 的光栅是灰度的——其清单中没有颜色的位置。
sstv-dither = 抖动
sstv-diffuse-quantisation-error-worth-below = 扩散量化误差——在低于 8 位时值得使用
sstv-caption = 说明
sstv-sent-manifest-content-hint-shown = 作为内容提示发送到清单中，并由接收方显示在图片下方。以文本形式传输——它不会绘制在图像中。
sstv-what-picture = 这张图片是什么
sstv-repeat-data = 重复数据
sstv-send-every-data-frame-many = 每个数据帧发送这么多次。RIFP 是单向的，没有修复请求，因此这是接收方唯一的恢复手段。
sstv-chunk = 数据块
sstv-payload-octets-per-data-frame = 每个数据帧的有效载荷字节数（该配置建议为 192）
sstv-valid-frames-frames-failed-crc = 有效帧、CRC 校验失败的帧，以及已完整验证的图片
sstv-no-transfer-progress = 没有正在进行的传输
sstv-forget-incomplete-transfer = 放弃此未完成的传输

## 记忆信道和语音键控器窗口（src/app/windows.rs）

windows-recall = 调用
windows-delete = 删除
windows-edit-name-frequency-mode-antenna = 编辑名称、频率、模式、天线和中继设置
windows-r = {" "}R
windows-repeater-shift-tone-1750-hz = 与此信道一同保存的中继偏移、音调和 1750 Hz 脉冲
windows-abandon-edit-escape = 放弃编辑（Esc）
windows-save = 保存
windows-keep-changes-enter = 保留更改（Enter）
windows-recall-channel-without-moving-antenna = 调用此信道，而不移动天线
windows-antenna-switch-when-channel-recalled = 调用此信道时要切换到的天线。— 保持电台当前所在的天线不变。
windows-shift = 偏移
windows-tone = 音调
windows-invert = 反相
windows-normal = 正常
windows-dcs-polarity = DCS 极性
windows-open-every-over-tone-burst = 调用此信道期间，每次通话都以音调脉冲打开中继
windows-memories = 记忆信道
windows-memory-name = 记忆名称
windows-store = 存储
windows-folder-name = 文件夹名称
windows-new-folder = 新建文件夹
windows-import = 导入
windows-read-channel-list-chirp-csv =
    从 CHIRP CSV 文件（.csv）读取信道列表——中继台目录导出、海事或 PMR 信道表，或 CHIRP 能够写出的任何其他内容。

    每个信道都会带上其频率、模式、中继偏移以及 CTCSS 或 DCS 音调。此列表中已有的信道——相同频率、相同模式——会被跳过，因此重新导入更新后的目录只会添加新内容，而不会使已有内容重复。
windows-chirp-csv = CHIRP CSV
windows-export = 导出
windows-write-list-out-chirp-csv = 将此列表写出为 CHIRP CSV 文件，用于载入手持机或作为备份保存。
windows-no-memories-yet = 尚无记忆信道
windows-drag-memory-onto-folder-file = 将记忆信道拖到文件夹上即可归档，拖到它们下方即可取消归档
windows-drop-here-unfile = 放在此处即取消归档
windows-sort = 排序
windows-order-they-were-stored = 按存储顺序
windows-name-ignoring-case = 按名称（不区分大小写）
windows-frequency = 按频率
windows-band-then-frequency-inside-each = 按频段，每个频段内再按频率
windows-descending-click-ascending = 降序——点击切换为升序
windows-ascending-click-descending = 升序——点击切换为降序
windows-delete-folder-memories-move-top = 删除文件夹——其中的记忆信道将移至顶层
windows-rename-folder = 重命名文件夹
windows-empty-drop-memories-here = 为空——将记忆信道拖到此处
windows-voice-keyer = 语音键控器
windows-rec-records-your-microphone-play = 录制从你的话筒录音，播放让你听到所录的内容，发射则将其送上空中——数字小键盘按键、MIDI 打击垫或 rigctld 的 send_voice_mem 同样可以触发。
windows-stop-store = 停止并存储
windows-stop = 停止
windows-play = 播放
windows-stop-listening = 停止收听
windows-nothing-recorded-slot = 此时隙中没有录音
windows-not-while-transmitting = 发射期间不可用
windows-listen-message-nothing-transmitted = 收听此消息——不会发射任何内容
windows-stop-transmitting = 停止发射
windows-tune-active-switch-off-first = 调谐处于激活状态——请先将其关闭
windows-transmit-message = 发射此消息
windows-erase-recording = 擦除此录音
windows-rade-message-encoded-digital-voice = RADE：消息由数字话音编解码器编码，与实时通话完全相同。

## 发射音频窗口：带数值的消息

txaudio-rec-hint = 从话筒录制一段测试短语——最长 { $max } 秒，无需按下发射机。再次点击即停止。它保持未处理状态，因此“播放”可以让你每次更改后都用同一段短语进行比较。
txaudio-phrase-length = { $secs } 秒短语
# $clip 为 txaudio-microphone-may-clipping-before-gets 或为空。
txaudio-auto-set-done = 输入增益已设为 { $gain } dB（话音峰值为 { $loud } dBFS）。{ $clip }
txaudio-listening = 正在收听…… { $secs } 秒
txaudio-auto-set-hint = 请像在空中那样正常说话 { $secs } 秒，输入增益将被设定为使你的话音峰值约为 { $target } dBFS。再次点击即取消。
txaudio-eq-point = { $hz } Hz，{ $db } dB——拖动可移动
txaudio-not-a-profile = 不是发射音频配置文件：{ $error }
txaudio-imported = 已导入“{ $name }”。
txaudio-saved = 已保存“{ $name }”。
txaudio-deleted = 已删除“{ $name }”。
txaudio-loaded-preset = 已加载“{ $name }”起始预设。

## 太阳系三维视图：带数值的消息

solar-age-offline = { $age } · 离线
solar-radius-exaggeration = 地球/月球半径夸大（在此月球轨道比例下最大为 { $max }×）
solar-brightest-cell = 最亮的方格约 { $peak } 条路径 · { $bands } 个频段
solar-hits-in-hour = 一小时内 { $hits } 次
solar-passes-from = { $name } · 来自 { $grid } 的过境
solar-elements-age = 轨道根数已过 { $age } · SGP4
solar-point-at = 指向方位 { $az }°（{ $compass }），仰角 { $el }°。
solar-published-as = 发布为 { $name }
solar-fly-there = { $name } — 按 ↵ 飞往该处
solar-sats-tracked = 已跟踪 { $total } 颗中的 { $n } 颗
solar-bodies-found = 已找到 { $total } 个天体中的 { $n } 个
solar-sats-and-bodies = 已跟踪 { $total } 颗中的 { $n } 颗 · { $bodies_total } 个天体中的 { $bodies } 个
solar-confidence-on-band = { $band } 上的 { $confidence }
solar-kp-now = 当前 Kp { $kp }
solar-kp-in = { $hours } 小时后 Kp { $kp }
solar-valid-age = 有效期 { $valid } · 已过 { $age }
solar-full-bar = 满条 = 全球的 { $share }
solar-own-paths-memory = 本电台自身路径的走向 · { $minutes } 分钟记忆
solar-n-storms = { $n } 次风暴
solar-clouds-legend = 云层 { $time } · 已过 { $age } · { $channels } · { $storms } · 闪电为模拟
solar-cme-eta = 预计 { $eta }（+{ $hours } 小时）
solar-cme-arrived = 已于 { $eta } 到达（{ $hours } 小时前）
# $glancing 和 $estimated 为 solar-glancing / solar-direction-estimated 或为空。
solar-cme-earth-directed = 指向地球的日冕物质抛射 { $time } · { $speed } km/s · { $when }{ $glancing }{ $estimated }

## 数字模式设置对话框：带数值的消息

setup-mode-setup = { $mode } 设置
setup-transmits-as = 以 { $call } 发射
setup-decimal-degrees = 十进制度，{ $hemisphere } 为负。可直接从地图上粘贴。

## 功放窗口：带数值的消息

amp-unreliable-at-swr = { $meters } 在此驻波比下不可靠
amp-lower-drive-first = 请先降低驱动：{ $drive } W 会使接入线路的功放过驱动（其限值为 { $limit } W）。
amp-live-value = 实时 { $value } { $unit }
amp-tripped = 已触发{ $why }

## 数字模式面板，共用部分：带数值的消息

panels-top-paths = 最多约 { $n } 条路径
panels-no-agreed-frequency = { $mode } 没有约定的频率——打开此面板以保存你当前所在的频率
panels-mode-frequencies-hint = { $mode } 的 { $n } 个频率——选择其中一个即可调谐表盘，你也可以保存自己的频率
panels-mode-frequencies = { $mode } · 频率
panels-global-convention = 一项全球惯例，但 IARU { $region } 区域的频段计划并未将窄带数据放在此处——在此发射前请核对你自己的频段计划。
panels-outside-data-segment = 琥珀色：位于 { $region } 区数据子频段之外。
panels-save-dial = ＋ 保存 { $mhz } MHz
panels-save-dial-hint = 以 { $mode } 记住此表盘频率，并从现在起在此处提供它。保存在电台站点上，因此连接到此电台的每个屏幕都有它。
panels-tr-period = { $period } 秒收发周期
# $mark 为正在发射时的 panels-burst-mark，否则为空。
panels-this-turn = 本轮：{ $slot } 秒时隙中的 { $into } 秒。解码结果在其末尾出现，而那正是下一次发射可以开始的时候。{ $mark }
panels-burst-mark = {" "}标记是脉冲停止的位置，位于 { $secs } 秒处。
panels-tr-period-burst = { $slot } 秒收发周期，{ $burst } 秒脉冲

## 记忆信道和语音键控器窗口：带数值的消息

windows-n-channels = { $n } 个信道
windows-slot-n = 时隙 { $n }
windows-record-from-mic = 从话筒录制（最长 { $max } 秒）

## SSTV 面板：带数值的消息

sstv-picture-too-big = 该图片为 { $size } MB——限制为 { $limit } MB。
sstv-auto-mode = 自动（{ $mode }）
sstv-not-decoded = { $what }——未解码
sstv-last = 上一个：{ $mode }
# $place 为 sstv-store-on-radio 或 sstv-store-in。
sstv-saved-where = 解码的图片保存{ $place }，并收集在此处。
sstv-older-in-store = 存储中另有 { $n } 张更早的图片
sstv-older-load-more = 另有 { $n } 张更早的图片——加载更多
sstv-delete-picture-from = 从{ $place }删除此图片
sstv-rifp-profile = { $profile } · 占用约 { $khz } kHz · 表盘为信道中心
sstv-rifp-too-wide = ⚠ { $profile } 占用约 { $khz } kHz——对于窄带段来说太宽
sstv-rifp-no-frequency = RIFP 不指定频率，sdroxide 会将其在你调谐到的任何地方发射。一个 { $khz } kHz 的信道只适用于允许宽带或 FM 操作的地方——{ $segments }——而不适用于窄带段，在 HF 上尤其如此。即使在这些地方，你自己的执照条件也可能更窄。你是操作员；请核对你自己的规则。
sstv-rifp-tx-frame = ● 发射帧 { $frame }/{ $frames } · 剩余 { $secs } 秒
sstv-rifp-sent-as = 以 { $encoding } 发送 · { $octets } 字节
sstv-rifp-rx-stats = 帧 { $frames } · 错误 { $bad } · 图片 { $pictures }
sstv-rifp-session-idle = 会话 { $session } · 空闲 { $secs } 秒
sstv-rifp-session-waiting = 会话 { $session } · 已保留数据块，仍在等待清单 · 空闲 { $secs } 秒
sstv-store-on-radio = 在电台上，位于 { $dir }
sstv-store-in = 位于 { $dir }

## 解码列表（src/app/panels/decodes.rs）

decodes-decodes = 解码
decodes-adif = ADIF
decodes-sort = 排序
decodes-single-list = 单一列表
decodes-every-decode-one-list-newest = 所有解码放在一个列表中，最新的时段在前，而不是按奇偶时段分块。此时“排序”按钮排列的是整个列表而非每个时段，并且每一行都带有其时隙时间——偶数时隙为青色，奇数时隙为金色。
decodes-cq-only = 仅 CQ
decodes-only-stations-calling-cq-only = 仅显示呼叫 CQ 的电台——并且只显示你可以应答的呼叫：定向 CQ（DX、EU、JA、POTA、TEST 等）在指名你时才会列出，指名他人时则会隐藏。
decodes-new-only = 仅新的
decodes-only-stations-would-new-entity = 仅显示会成为新内容的电台：实体、频段时隙、网格或呼号
decodes-auto-tx-frq = 自动发射频率
decodes-overridden-hold-tx-lift-hold = 已被“保持发射”覆盖。请取消保持，以选择发射频率的移动方向。
decodes-pick-our-transmit-frequency-automatically = 自动选择我们的发射频率：在我们发射的时段中最安静的位置，而不是我们所应答的电台的频率——对方在另一个时段发射，因此他们的频率说明不了我们按下发射时那里有谁。关闭时并不会保持该频率：它会在被呼叫电台的频率上应答。若要保持，请使用“保持发射”。
decodes-hold-tx = 保持发射
decodes-pin-transmit-tone-where-nothing =
    将发射音调固定在当前位置。没有任何东西会移动它：应答电台、呼叫队列、呼叫 CQ、点击解码或瀑布图都不会。若要移动，请先将其关闭，然后再开启。

    更换频段是唯一的例外，而且这是你自己的操作：你在新频段上最后设定的偏移会随之恢复，因此 60 米的数值不会被带到 20 米上，20 米的数值也不会被带到 60 米上。

    若你的执照比频段计划更窄——例如在英国 60 米的 5357 kHz 表盘上，分配在 5358.0 结束，因此音调必须保持在 1000 Hz 以下，而两种自动移动方式都会在一次通话与下一次通话之间移出频段。
decodes-type-transmit-offset-hz-then = 以赫兹输入发射偏移，然后按 Enter。若你的执照比频段计划更窄，则边缘以下的整个范围都由你自己选择，因此此处不提供任何建议。
decodes-clear-rx = {" "}清空接收{" "}
decodes-empty-decode-list-nothing-air = 清空解码列表。空中的任何内容都不会停止。
decodes-even = 偶数
decodes-text = 文本
decodes-dxcc = DXCC
decodes-band = 频段
decodes-grid = 网格
decodes-dupe = 重复
decodes-reply = 应答
decodes-queued-click-remove = 已排队——点击即可移除
decodes-work-station-after-current-one = 在当前电台之后与此电台通联
decodes-qsos-worked-since-sdroxide-was = 自 sdroxide 启动以来完成的通联
decodes-setup = ⚙ 设置
decodes-qso-complete = ✓ 通联完成
decodes-contact-complete-log-held-here = 通联已完成并记入日志。它会在此保留几分钟，以便在对方重复其最后一条消息时可以重新发送——不再有其他需要处理的事项。
decodes-tx = ● 发射
decodes-dxpedition-mode-transmit-frequency-held = DXpedition 模式。发射频率被保持在通带中 Fox 所占的那一半之外（1000 Hz 以下），直到 Fox 应答为止。
decodes-watchdog = 看门狗
decodes-transmitting-stopped-no-reply-no = 发射已停止：在看门狗周期内既无回复也无操作。请呼叫 CQ 或选择一条消息以恢复。
decodes-well-inside-tolerance = 完全在容差范围之内。
decodes-no-active-qso-pick-decode = 没有进行中的通联——请选择一条解码以应答，或呼叫 CQ
decodes-ft8-engine-idle = FT8 引擎空闲
decodes-being-worked = 正在通联
decodes-clear = 清除
decodes-no-messages = — 无消息 —
decodes-call-cq = {"  "}呼叫 CQ{"  "}
decodes-stop-qso = {" "}停止通联{" "}
decodes-stop-tx = {" "}停止发射{" "}
decodes-free-text-13-chars = 自由文本（13 个字符）
decodes-send = 发送

## 卫星（src/app/sat.rs）

sat-pass-profile = 过境配置
sat-satellite = 卫星
sat-satellites = 卫星
sat-passes = 过境
sat-set-your-grid-passes-settings = 请设置你的网格以计算过境（设置 ▸ 常规）。
sat-next = 下一次
sat-peak = 峰值 ≥
sat-all-satellites = 所有卫星
sat-off-only-satellites-frequencies-file = 关闭：仅包含已存档频率的卫星——即你能够通联的那些——以及你自己粘贴的任何卫星。开启：列表中的每一颗卫星，若有订阅，可能多达数千颗，搜索需要一些时间。
sat-no-passes-window-lower-peak = 此窗口内没有过境。请降低峰值，或加宽窗口。
sat-length = 长度
sat-peak-2 = 峰值
sat-from-to = 自 → 至
sat-up-now = ● 当前在空
sat-unlock = 解锁
sat-release-lock-dial-stays-where = 释放锁定；表盘保持在原位
sat-azimuth = 方位角
sat-elevation = 仰角
sat-range = 距离
sat-rate = 速率
sat-downlink = 下行
sat-doppler-rx = 接收多普勒
sat-uplink = 上行
sat-doppler-tx = 发射多普勒
sat-no-pass-inside-next-48 = 从你的 QTH 看，未来 48 小时内没有过境。
sat-elements-stale-corrections-suspended-until = ⚠ 轨道根数已过时——在 TLE 刷新之前，校正已暂停
sat-doppler-correction-needs-iq-front = 多普勒校正需要 IQ 前端——CAT 电台只能跟踪
sat-rotator-not-connected = 旋转器：未连接
sat-doppler = 多普勒
sat-apply-computed-correction-rx-tx = 将计算出的校正应用于接收和发射
sat-rotator = 旋转器
sat-steer-antenna-through-rotctld-client = 通过 rotctld 客户端操控天线（设置 ▸ 服务器）
sat-name-norad = 名称或 NORAD
sat-set-your-grid-passes-settings-2 = 设置你的网格以计算过境（设置 ▸ 常规）
sat-norad = NORAD
sat-pass = 过境
sat-always-visible = 始终可见
sat-not-here = 此处不可见
sat-no-elements = 无轨道根数
sat-pick-satellite-see-links-lock = 选择一颗卫星，以查看其链路并锁定。
sat-no-frequencies-file-one-add = 此卫星没有已存档的频率——请在“设置 ▸ TLE”中添加。
sat-link = 链路
sat-downlink-mhz = 下行（MHz）
sat-uplink-mhz = 上行（MHz）
sat-mode = 模式
sat-tune = 调谐
sat-set-dial-mode-link-no = 将表盘和模式设为此链路——不锁定，不校正
sat-locked = {" "}已锁定{" "}
sat-lock = {" "}锁定{" "}
sat-track-satellite-doppler-correction-rx = 跟踪此卫星：对接收和发射进行多普勒校正，上行按转发器映射，三维视图随之跟随

## WEFAX（src/app/panels/wefax.rs）

wefax-wefax = WEFAX
wefax-stop = {" "}■ 停止{" "}
wefax-end-chart-now-save-what = 立即结束图表，并保存已接收的内容
wefax-start = {" "}● 开始{" "}
wefax-start-chart-now-without-waiting = 立即开始一张图表，无需等待开始音
wefax-phasing = 对相……
wefax-subcarrier-offset-1900-hz-tune = 相对于 1900 Hz 的副载波偏移。请调谐至大约为零：传真载波为 1500 Hz（黑）至 2300 Hz（白），若接收机偏差数百赫兹，图片就会被削成全黑或全白。
wefax-auto-start = 自动开始
wefax-begin-chart-when-300-hz = 当听到 300 Hz（IOC 576）或 675 Hz 开始音时开始一张图表——并在下一次发射到达时结束正在进行的图表并对新图表进行对相，无论之前是否收到停止音。
wefax-auto-stop = 自动停止
wefax-end-450-hz-stop-tone = 在 450 Hz 停止音时结束。关闭后，对于连续发送多张图表的电台，会一直录制下去。
wefax-phase = 相位
wefax-slant = 倾斜
wefax-sample-clock-trim-if-chart = 采样时钟微调。若图表向左倾斜，请调高此值；若向右倾斜，则调低。时钟偏差一百 ppm 的声卡，在一张十五分钟的图表上会使大部分行发生横向偏移。
wefax-view = 视图
wefax-scale-chart-panel-width = 将图表缩放到面板宽度
wefax-whole = 整体
wefax-shrink-chart-until-all-view = 缩小图表，直至其全部同时可见
wefax-half-size = 半尺寸
wefax-one-screen-pixel-per-fax = 每个传真像素对应一个屏幕像素——滚动以查看细节
wefax-twice-size-scroll-move-around = 两倍尺寸；滚动以在图表中移动
wefax-height = 高度
wefax-stretch-picture-vertically-chart-came = 垂直拉伸图片。若图表被压扁或拉长，通常是因为解码时使用的行速率有误——拉伸可使其可读，而 LPM 芯片则能从根本上解决此问题。双击可输入数值。
wefax-reset = 重置
wefax-back-picture-s-own-proportions = 恢复为图片自身的比例
wefax-follow = 跟随
wefax-keep-newest-lines-view-scrolling = 使最新的行保持在视图中。向上滚动会关闭此项，以便你阅读已经接收到的内容；滚动回底部则会再次开启。
wefax-tune-fax-schedule-usb-wait = 在 USB 上调谐到传真时刻表并等待开始音，或按“开始”从图表中间开始。
wefax-saved = 已保存
wefax-path = 路径
wefax-reading-radio-s-charts = 正在读取电台的图表……
wefax-finished-charts-collect-here = 完成的图表会收集在这里。
wefax-delete-chart = 删除此图表
wefax-loading = 加载中……
wefax-stations = 电台
wefax-broadcast-radiofax-schedules-picking-one = 广播无线电传真时刻表——选择其中一项即可调谐表盘
wefax-frequencies-are-published-carrier-dial = 频率为公布的载波；表盘位于其下方 1.9 kHz，此操作已为你完成。时刻表会变化，电台也会停播——请将其作为开始查找的地方，而不是时间表。
wefax-delete-chart-store = 从存储中删除此图表
wefax-newer = ◀ 较新
wefax-chart-received-after-one = 在此之后接收的图表
wefax-older = 较早 ▶
wefax-chart-received-before-one = 在此之前接收的图表
wefax-no-longer-store = 已不在存储中
wefax-loading-full-size = 正在加载原始尺寸……
wefax-save = 保存
wefax-save-copy-chart-computer = 在本计算机上保存此图表的副本
wefax-sure = 确定？
wefax-click-again-delete-chart-good = 再次点击即永久删除此图表
wefax-delete = 删除
wefax-file-chart-was-saved = 保存此图表所用的文件

## APRS（src/app/panels/aprs.rs）

aprs-starting-aprs-modem = 正在启动 APRS 调制解调器……
aprs-aprs = APRS
aprs-no-callsign = 无呼号
aprs-nothing-will-transmitted-no-beacon = 在此电台拥有呼号之前，不会发射任何内容——不发信标，不发消息，甚至不发确认。请在“设置 → 常规”中设置呼号，或在“设置”中为 APRS 单独设置带 SSID 的呼号。
aprs-no-position = 无位置
aprs-beacon-needs-somewhere-report-fill = 信标需要一个报告位置。请在“设置 → 常规”中填入你的网格定位，或在“设置”中给出坐标。
aprs-setup = 设置
aprs-callsign-symbol-digipeater-path-position = 呼号、符号、数字中继路径、位置和信标
aprs-how-often-beacon-your-position = 无人值守时，多久发送一次你的位置信标。`off`——默认值——从不发送信标：选择某种模式不得使电台上空。三十分钟是固定电台的惯例，移动电台则更频繁，而且每一次信标都占用别人的信道时间。
aprs-beacon = {" "}信标{" "}
aprs-station-has-no-callsign-see = 此电台没有呼号——请参见左侧的警告。
aprs-station-has-no-position-report = 此电台没有可报告的位置——请参见左侧的警告。
aprs-send-one-position-report-now = 立即发送一次位置报告，无需等待间隔。与其他所有内容一样，它会等待信道空闲。
aprs-stations = 电台
aprs-nothing-heard-yet = 尚未听到任何内容
aprs-heard-direct = 直接收听到
aprs-two-characters-station-sent-say = 电台发送的两个字符，用于说明它是什么
aprs-killed = 已撤销
aprs-station-put-object-here-has = 在此放置该对象的电台已将其撤销
aprs-sender-deliberately-reported-square-rather = 发送方特意报告的是一个方格，而非一个点。地图绘制的是方格。
aprs-digipeaters-last-frame-came-through = 最后一帧经过的数字中继。带有 `*` 的表示它确实转发了该帧。
aprs-message = {" "}消息{" "}
aprs-address-message-box-station = 将消息框的收信人设为此电台
aprs-center = {" "}居中{" "}
aprs-put-station-middle-map-double = 将此电台置于地图中央。双击地图即可将视图交还给自动适配。
aprs-channel = 信道
aprs-messages = 消息
aprs-frames-arrived-failed-their-check = 已到达但未通过校验序列的帧——可能是碰撞、衰落，或信号太弱而无法读取。计数上升而没有解码结果，说明调制解调器*确实*在收听该信道：请检查电平表，并打开电台的静噪。
aprs-frames-read-cleanly-off-channel = 从信道上干净读取、但并非 APRS 的帧——可能是他人的分组通信，或本版本无法解码的格式。
aprs-every-frame-channel-arrived-other = 信道上到达的每一帧，按其到达的原样显示——包括他人的通信，以及解码器无法读取的任何内容。
aprs-nothing-channel-yet = 信道上尚无任何内容
aprs-no-messages = 没有消息。
aprs-waiting-channel = 等待信道
aprs-sent-retried-still-waiting-acknowledged = 已发送并重试，仍在等待确认
aprs-air-waiting-acknowledged = 已在空中，等待确认
aprs-acknowledged-far-end = 已被对方确认
aprs-refused-far-end-not-retried = 被对方拒绝——不再重试
aprs-no-ack = 无确认
aprs-no-answer-after-every-retry = 每次重试后均无应答
aprs-send = {" "}发送{" "}
aprs-station-has-no-callsign-so = 此电台没有呼号，因此无法发射任何内容。请在“设置 → 常规”中设置呼号，或在“设置”中为 APRS 单独设置带 SSID 的呼号。
aprs-needs-station-address-something-say = 需要一个收信电台和要说的内容。
aprs-send-keep-retrying-until-far = 发送它，并持续重试，直到对方确认。消息是此信道上唯一会被应答的内容。
aprs-busy = 忙
aprs-stations-heard-still-inside-window = 在 APRS 设置中设定的窗口内听到的电台，以及此刻是否有其他电台在信道上。忙碌时将不会键控任何内容。
aprs-tx = ● 发射
aprs-over-progress-frames-waiting-channel = 正在进行的通话、等待信道空闲的帧，或距离下一次计划信标的时间。
aprs-audio-reaching-modem-flat-means = 到达调制解调器的音频。平直表示完全没有任何东西到达——请检查电台的数据输出是否就是 sdroxide 所收听的那一路。在调制解调器听到载波时，它会亮起绿色。

## VDL2（src/app/panels/vdl2.rs）

vdl2-starting-vdl2-decoder = 正在启动 VDL2 解码器……
vdl2-aircraft-datalink = 飞机数据链
vdl2-tune-middle-vdl2-group-decoder = 调谐到 VDL2 组的中心。解码器自己的窗口会从那里滑动，以尽可能多地纳入接收机能够触及的十四个信道。
vdl2-setup = 设置
vdl2-which-channels-listen-how-hard = 收听哪些信道、突发信号需要多强，以及保留多少日志
vdl2-save-vdl2-message-log-file = 将 VDL2 消息日志保存到文件
vdl2-vdl2-channel = VDL2 信道
vdl2-messages = 消息
vdl2-address-flight-registration-label = 地址、航班、注册号或标签
vdl2-held = 已暂停
vdl2-hold = 暂停
vdl2-hold-message-log-where-so = 将消息日志保持在原位，以便在下一次发射将其推上屏幕之前阅读其中一条。解码器继续运行，计数器也继续变化；松开后即可显示此期间到达的所有内容。
vdl2-nothing-decoded-yet = 尚无任何解码内容
vdl2-nothing-matches-filter = 没有任何内容符合过滤条件
vdl2-stations = 电台
vdl2-nobody-heard-yet = 尚未听到任何电台
vdl2-vdl2-setup = VDL2 设置
vdl2-channels = 信道
vdl2-every-25-khz-slot-136 = 从 136.650 到 136.975 MHz，每个 25 kHz 的时隙。每个信道各有一个下变频器，全部位于同一个接收机窗口之内。关闭某一个会节省一点处理器时间；它并不会使其他信道更灵敏。
vdl2-burst-threshold = 突发阈值
vdl2-above-each-channel-s-own = 高于每个信道各自的底噪
vdl2-lower-catches-weaker-transmissions-costs = 更低的阈值能捕获更弱的发射，但会在噪声上耗费处理器时间；更高则会漏掉它们。底噪是按信道逐一学习的，在此处的更改不会将其丢弃。
vdl2-keep-log = 保留在日志中
vdl2-messages-2 = {" "}条消息
vdl2-track-most = 最多跟踪
vdl2-stations-2 = {" "}个电台
vdl2-longest-silent-go-first = 最长时间保持沉默的先移除
vdl2-forget-station-after = 在以下情况下忘记一个电台
vdl2-nothing-heard = 且未从其收到任何内容
vdl2-show-unread-payloads = 显示未读载荷
vdl2-hex = 以十六进制显示
vdl2-frames-carrying-x-25-clnp = 承载 X.25、CLNP 或其之上的数据链应用的帧。SDRoxide 会为它们命名并显示字节，而不是读取它们；若将其隐藏，就会隐藏这些流量所占的比例。
vdl2-registration = 注册号
vdl2-flight = 航班
vdl2-label = 标签
vdl2-block = 块
vdl2-sequence = 序号
vdl2-mode = 模式
vdl2-more-blocks-follow = 后续还有更多块
vdl2-check = 校验
vdl2-message-crc-good = 消息 CRC 正确
vdl2-message-crc-not-checked = 未检查消息 CRC
vdl2-parity = 奇偶校验
vdl2-link-control-no-payload = 链路控制，无载荷
vdl2-name = 名称
vdl2-addr = 地址
vdl2-type = 类型
vdl2-msgs = 消息数

## CW（src/app/panels/cw.rs）

cw-tone-being-copied-tone-transmitted = 被抄收的音调和所发射的音调——在 CW 中它们是同一频率。点击瀑布图，可将其移到一个信号上。
cw-down-10-hz = 降低 10 Hz
cw-up-10-hz = 提高 10 Hz
cw-qrg-main-readout-tuning-line = QRG：主读数和调谐线位于正在通联的频率上。点击则改为显示表盘——即低于它的侧音音高，这是大多数电台的显示方式。
cw-put-main-readout-tuning-line =
    将主读数和调谐线置于信号上，而不是表盘上，因此所显示的频率就是双方操作员都会报出的频率，而调谐线位于通带中央。调谐方式不变；只是数字发生了移动。

    点击信号时，其落点只能精确到点击步进所允许的程度——控制 → 点击调谐取整，默认为 10 Hz。粗糙的步进会使信号偏离音高达其一半，读数中会对此加以说明。
cw-sending-speed-read-off-signal = 从信号读取的发送速度
cw-signal-noise-500-hz-same = 500 Hz 带宽内的信噪比——与报告中所引用的数值相同
cw-how-far-off-cursor-signal = 信号实际偏离光标多远。解码器会跟随它；通带则不会，因此若偏差增大，请轻推光标。
cw-listening = — 收听中 —
cw-tx = ● 发射
cw-trainer = 训练器
cw-learn-morse-read-characters-words = 学习摩尔斯电码：阅读字符和单词，以你自己的速度播放它们，还有一次添加一个字符的 Koch 练习。它通过本计算机的扬声器播放——不会键控电台。
cw-nothing-copied-yet = — 尚未抄收任何内容 —
cw-key-send-characters-you-send = 按键发送——你发送的字符会出现在此处。
cw-type-line-return-sends = 输入一行，按回车发送……
cw-type-here-send = 在此输入以发送……
cw-key = {" "}键 ●{" "}
cw-key-2 = {" "}键{" "}
cw-hold-key-bound-cw-straight = 将绑定到 CW 直键的按键——默认为空格键，你也可以在“设置 → 控制”中选择任何按键——作为直键按住：按住时按下，松开时抬起，而不是输入文字。开启时发射框被锁定，整个键控器交给直键使用：排队中的任何文字都会被丢弃。
cw-radio-sends-own-keyer-text =
    此电台由其自己的键控器发送：文字通过控制端口发出，电台负责计算各个元素的时序，因此在直键和空中之间没有任何东西可供手来驱动。

    若要手动键控，请在“设置 → 电台”中将 CW 键控设为“声卡（MCW）”。此时电台会保持在某个边带上，键控器自身的侧音以音频方式发射，这正是直键所驱动的通路。
cw-watchdog = 看门狗
cw-straight-key-was-held-down = 直键被按住的时间过长——是抬起信号丢失，而不是手在操作——因此载波已被切断，发射也已关闭。再次按下键即可继续。
cw-tx-2 = {"  "}发射开{"  "}
cw-tx-3 = {"   "}发射{"   "}
cw-send-what-box-now-without = 立即发送输入框中的内容，无需等待回车
cw-hold-key-down-between-characters = 在字符之间保持按键按下，这样键入的内容都无需等待
cw-call-cq = {" "}呼叫 CQ{" "}
cw-nocall = 无呼号
cw-clear = {" "}清除{" "}
cw-stop-sending-drop-whatever-has = 停止发送，并丢弃尚未发出的内容。开启直键时，对你所键入内容的回读也会一并停止。
cw-sidetone = 侧音
cw-play-keyed-tone-through-computer = 在发送的同时，也通过本计算机的扬声器播放所键控的音调，以便你听到自己正在发送的内容。在“声卡（MCW）”下，音调只输出到电台，别处听不到，因此没有此项你就是在无声地发送。若电台自身的监听或离线抄收已能完成此任务，则请关闭。
cw-hold-what-typed-until-return = 将键入的内容保留到按下回车，然后整行一次性发送，而不是在键入每个字符时就逐个键控到空中。这在能够由文字自行键控的收发信机上值得开启，因为每一次交给其键控器的操作，都是一次完整的收发循环。
cw-your-own-message-buttons-contest = 你自己的消息按钮——比赛交换、姓名和 QTH 的回复、TNX 73 GL。每个按钮都会一次性发送其全部文字，F2–F10 则按下前九个。它们随电台站点的配置一同传送，因此远程客户端也能使用。
cw-cw-messages = CW 消息
cw-clear-rx = {" "}清空接收{" "}
cw-empty-receive-window-straight-key = 清空接收窗口以及直键的回读内容。空中的任何内容都不会停止。
cw-lock = 锁定
cw-decode-speed-set-here-instead = 以此处设置的速度解码，而不是从信号中读取速度。有助于处理太弱、以至于速度搜索无法稳定下来的信号。
cw-farnsworth-send-characters-full-speed = Farnsworth：以全速发送字符，只拉长它们之间的间隔，使其以正确的节奏被听到，同时又慢到足以记下。
cw-off-normal-spacing = 关闭——正常间距
cw-keying-speed = 键控速度
cw-idle-0 = 空闲 0
cw-how-long-transmit-held-after = 在最后一个字符之后，或松开直键之后，发射会保持多久才切断载波。更长可以容纳慢速打字者的停顿；更短则能更快地离开该频率；关闭则在排队的所有内容发出后立即切断。
cw-off-drop-once = 关闭——立即切断

## 文本模式（src/app/panels/text_modem.rs）

textmodem-tune-down-10-hz = 向下调谐 10 Hz
textmodem-tune-up-10-hz = 向上调谐 10 Hz
textmodem-tx = ● 发射
textmodem-listening = — 收听中 —
textmodem-type-line-return-sends = 输入一行，按回车发送……
textmodem-type-here-transmit = 在此输入以发射……
textmodem-tx-2 = {"  "}发射开{"  "}
textmodem-tx-3 = {"   "}发射{"   "}
textmodem-call-cq = {" "}呼叫 CQ{" "}
textmodem-nocall = 无呼号
textmodem-clear = {" "}清除{" "}
textmodem-hold-what-typed-until-return = 将键入的内容保留到按下回车，然后整行一次性发送，而不是在键入每个字符时就将其逐一推送到空中。这使得一行内容可以在任何部分发出之前被回读并更正。
textmodem-your-own-message-buttons-working = 你自己的消息按钮——工作条件、天气、标准回复。每个按钮都会一次性发送其全部文字，F2–F10 则按下前九个。它们随电台站点的配置一同传送，因此远程客户端也能使用。
textmodem-messages = 消息
textmodem-hell = HELL
textmodem-contrast = 对比度
textmodem-harder-softer-dots-redraws-whole = 更硬或更软的点——重绘整个条带
textmodem-width = 宽度
textmodem-t-2row = {" "}2 行{" "}
textmodem-draw-every-column-twice-stacked = 将每一列绘制两次，上下叠放。Hell 没有垂直同步，因此无论相位如何，这都能保持一份完整的文字副本可读。
textmodem-rev = {" "}反色{" "}
textmodem-reverse-video-light-dots-dark = 反色显示——深色纸上的浅色点
textmodem-clear-rx = {" "}清空接收{" "}
textmodem-hold-channel-idle-sends-blank = 保持信道：空闲时发送空白纸，使条带继续滚动
textmodem-hold-what-typed-until-return-2 = 将键入的内容保留到按下回车，然后整行一次性发送，而不是在键入每个字符时就将其逐一绘制到条带上。实时键入的更正已经在纸上；而在回车之前所做的更正则从未出现在纸上。
textmodem-shift = 频移
textmodem-shift-5-hz = 频移 −5 Hz
textmodem-shift-5-hz-2 = 频移 +5 Hz
textmodem-baud = 波特率
textmodem-reverse-swap-mark-space-needed = 反向：交换标记和空号。当信号在与发送时相反的边带上被接收、从而使音调颠倒时，需要使用此项。
textmodem-track-tuning-error-automatically-instead = 自动跟踪调谐误差，而不是固定在光标上。
textmodem-tones = 音调
textmodem-mode = 模式
textmodem-off-keyed = 开/关键控

## AIS（src/app/panels/ais.rs）

ais-starting-ais-decoder = 正在启动 AIS 解码器……
ais-tune-between-two-ais-channels = 调谐在两个 AIS 信道之间——161.975（AIS 1，海事 87B）和 162.025（AIS 2，88B）。表盘位于中间，因为那里没有任何发射，因此零中频接收机的直流尖峰不会落在任何一个信道上。
ais-setup = 设置
ais-channels-timeouts-trail-length-how = 信道、超时、轨迹长度，以及矢量延伸的距离
ais-vessels = 船舶
ais-nothing-heard-yet = 尚未听到任何内容
ais-virtual-nothing-there = 虚拟——那里并无实物
ais-position-stale = 位置已过时
ais-search-rescue-transmitter-man-overboard = 搜救发射机、落水信标或 AIS EPIRB
ais-call-sign = 呼号
ais-station-time = 电台时间
ais-dgnss = {" "}（DGNSS）
ais-first-heard = 首次收听
ais-center = 居中
ais-put-vessel-middle-chart = 将此船舶置于海图中央
ais-last-message-station-nmea-sentences = 此电台最后一条消息，以接收机会发送到线路上的 NMEA 语句形式显示。可将其粘贴到任何其他 AIS 解码器中，以核对 sdroxide 对它的解读。
ais-ais-setup = AIS 设置
ais-channels = 信道
ais-drop-chart-after = 超过以下时间后从海图中移除
ais-without-position-report = 没有位置报告
ais-five-minutes-default-not-ads = 默认为五分钟，而非 ADS-B 的十秒：锚泊中的船只每三分钟报告一次，而较短的窗口会使港口的大部分区域在两条完全有效的报告之间变为空白。超过此时间后，船舶会从海图上移除，其行会变灰——它不是淡出，因为位置过时的暗淡符号仍然是关于船舶位置的一种声明，与真实的符号使用相同的墨色。
ais-drop-list-after = 超过以下时间后从列表中移除
ais-nothing-heard-all = 完全没有听到任何内容
ais-message-carrying-ship-s-name = 携带船名的消息每六分钟才会出现一次，因此较短的列表窗口会在船舶刚要说出其名字之前就把它们丢弃。
ais-trail-length = 轨迹长度
ais-history-behind-each-target = 每个目标身后的历史轨迹
ais-minutes-rather-than-points-because = 以分钟而非点数计算，因为 AIS 的报告速率跨越两个数量级：固定的点数对渡轮而言是八十秒，对锚泊的油轮却是两小时，而绘制出来完全相同。设为零则关闭轨迹。
ais-speed-vector = 速度矢量
ais-how-far-ahead-vector-reaches = 矢量延伸的距离
ais-slot-threshold = 时隙阈值
ais-above-channel-s-learned-noise = 高于该信道已学习到的底噪
ais-track-most = 最多跟踪
ais-vessels-2 = {" "}艘船舶
ais-ais-receive-only-here-always = 此处的 AIS 仅用于接收，并且将始终如此：它是一项保障生命安全的服务，而执照并不允许在其上发送虚假的船舶通信。
ais-fill-my-grid-digimode-setup = 请在数字模式设置中填写“我的网格”，以便计算距离和方位，并使海图以你所在的位置为中心，而不是以最远的目标为中心。
ais-name = 名称
ais-mmsi = MMSI
ais-type = 类型

## qo100（src/app/qo100.rs）

qo100-receiver-cannot-reach-10489-750 = 此接收机自身无法到达 10489.750 MHz——请先设置 LNB/变频器偏移（设置 ▸ 电台）
qo100-run-fast-spectral-tracker-every = 运行快速频谱跟踪器：每秒在停靠窗口中查找信标的两个对称波瓣，并报告载波所在的位置
qo100-telemetry = 遥测
qo100-also-run-ao-40-uncoded = 同时运行 AO-40 未编码帧解码器——同步字、CRC 和遥测文本——并通过逐步显示（载波 → 同步 → CRC）读出每次过境的进展程度
qo100-auto = 自动
qo100-auto-correct-let-tracker-adjust = 自动校正：让跟踪器自行调整变频器/LNB 偏移——这是一个缓慢的闭环，在估计值干净且稳定时，它会将信标轻推回 10489.750 MHz，并在 LNB 漂移时保持其在那里。每次动作时都会重新打开前端。
qo100-narrower-search-smaller-slice = 更窄——搜索较小的片段
qo100-wider-when-beacon-isn-t = 更宽——用于当前宽度下找不到信标时
qo100-clear-mark = 清除标记
qo100-drop-hand-placed-beacon-mark = 删除手动放置的信标标记，并恢复为解码器自己的读数
qo100-lo = 低 −
qo100-lo-2 = 低 +
qo100-hi = 高 −
qo100-hi-2 = 高 +
qo100-radio-s-own-tuning-range = 此电台自身的调谐范围无法到达 10489.750 MHz——在此窗口能做任何事情之前，它需要 LNB/变频器偏移（设置 ▸ 电台 ▸ 变频器）。
qo100-open-settings-radio = 打开设置 ▸ 电台
qo100-tune-10489-750-mhz = 调谐到 10489.750 MHz
qo100-tip-if-you-can-see = 提示：如果你能在条带中看到信标但无法锁定，请双击它——这会将其标记为 10489.750 MHz，并让“应用”校正到该频率
qo100-tracker = 跟踪器
qo100-no-twin-lobe-shape-parking = 停靠窗口中没有双波瓣形状
qo100-shape = 形状
qo100-drift = 漂移
qo100-need-longer-run-estimates = —（需要更长的估计序列）
qo100-cycles = 周期
qo100-receiver = 接收机
qo100-target = 目标
qo100-measured = 测得
qo100-clicked = {"  "}（已点击）
qo100-tracker-2 = {"  "}（跟踪器）
qo100-not-locked-yet = 尚未锁定
qo100-converter-offset = 变频器偏移
qo100-decode = 解码
qo100-decode-filling-first-frame-buffer = 解码——正在填充第一个帧缓冲……
qo100-decode-off-turn-telemetry-chip = 解码——关闭。请开启遥测芯片（顶行）以查看载波 → 同步 → CRC 的读数。
qo100-apply-correction = {" "}应用校正{" "}
qo100-write-converter-lnb-offset-puts = 写入变频器/LNB 偏移，使信标位于你所点击的位置并对应到 10489.750 MHz，然后重新打开接收机——与“设置 ▸ 电台 ▸ 应用”所造成的短暂中断相同
qo100-write-converter-lnb-offset-puts-2 = 写入变频器/LNB 偏移，使跟踪器的估计值对应到 10489.750 MHz，然后重新打开接收机——与“设置 ▸ 电台 ▸ 应用”所造成的短暂中断相同
qo100-waiting-second-crc-valid-frame = 等待第二个 CRC 校验通过的帧，然后才提供写入此项——仅一次锁定可能只是偶然匹配
qo100-write-corrected-converter-lnb-offset = 写入校正后的变频器/LNB 偏移并重新打开接收机——短暂中断，与“设置 ▸ 电台 ▸ 应用”所造成的相同
qo100-decoder-runs-receiving-station-readout = 解码器在接收电台上运行——其读数不会发送给远程客户端
qo100-starting = 正在启动……
qo100-searching-first-window-fills-after = 正在搜索——第一个窗口约 24 秒后填满，然后重复

## JS8（src/app/panels/js8.rs）

js8-multi = 多速率
js8-decode-every-js8-speed-not = 解码所有 JS8 速率，而不仅是你发射所用的那一种。四种速率共用子频段，且是四种不同的波形，因此若不开启此项，以其他速率发送的电台将不可见——而两种速率之间的通信则根本无法发生。大约会使接收 CPU 开销增加四倍。
js8-hb-auto = 心跳自动
js8-turbo-does-not-beacon-local = Turbo 不发送信标——它是本地和 VHF 的速率
js8-beacon-your-callsign-grid-every = 每 15 分钟发送一次你的呼号和网格信标
js8-until-next-heartbeat = 直到下一次心跳
js8-setup = ⚙ 设置
js8-tx = ● 发射
js8-heard = 收听
js8-nothing-heard-yet = — 尚未听到任何内容 —
js8-dxcc = DXCC
js8-band = 频段
js8-grid = 网格
js8-dupe = 重复
js8-reply = 应答
js8-no-messages = — 无消息 —
js8-cq = {" "}CQ{" "}
js8-hb = {" "}心跳{" "}
js8-clear = {" "}清空至{" "}
js8-forget-selected-station-composer-goes = 忘记所选电台——编写框将回到 @ALLCALL
js8-already-addressing-allcall = 已在发送给 @ALLCALL
js8-stop = {" "}停止{" "}
js8-send = {" "}发送{" "}
js8-message = 消息……

## 日志（src/app/logbook.rs）

palette-hint = 前往……模式、频段、窗口、设置、记忆或频率（14.074）
palette-nothing = 没有此名称的项目
palette-group-window = 窗口
palette-group-settings = 设置
palette-group-memory = 记忆
palette-group-tune = 调谐
palette-tune-to = 调谐到 { $mhz } MHz
palette-win-bands = 频段条件
palette-win-awards = 奖项
palette-win-windows = 窗口列表
logbook-logbook = 日志
logbook-new-entry = + 新建条目
logbook-adif = ADIF
logbook-import = 导入
logbook-import-qsos-adif-adi-adif = 从 ADIF（.adi 或 .adif）文件导入通联
logbook-new-qso = 新建通联
logbook-edit-qso = 编辑通联
logbook-call = 呼号
logbook-lookup = 查询
logbook-look-up-name-qth-grid = 查询姓名 / QTH / 网格
logbook-grid = 网格
logbook-freq-mhz = 频率 MHz
logbook-mode = 模式
logbook-rst-sent = 发送 RST
logbook-rst-rcvd = 接收 RST
logbook-name = 姓名
logbook-state = 州 / 省
logbook-country = 国家
logbook-date-utc = 日期 UTC
logbook-time = 时间
logbook-pwr-w = 功率 W
logbook-contest = 比赛
logbook-s-sent = 发送序号
logbook-s-rcvd = 接收序号
logbook-comment = 备注
logbook-save = {" "}保存{" "}
logbook-cancel = 取消
logbook-lotw = LoTW ✓
logbook-eqsl = eQSL ✓
logbook-card = QSL 卡 ✓
logbook-lotw-2 = LoTW ↑
logbook-eqsl-2 = eQSL ↑
logbook-qrz = QRZ ↑
logbook-hamqth = HamQTH ↑
logbook-club-log = Club Log ↑
logbook-wavelog = Wavelog ↑
logbook-delete-entry = 删除此条目
logbook-edit = 编辑
logbook-upload-qso-configured-logs = 将此通联上传到已配置的日志
logbook-no-qsos-yet-run-ft8 = 尚无通联——请运行 FT8/FT4，或添加手动条目

## 扫描器（src/app/scanner.rs）

scanner-scanner = 扫描器
scanner-work-through-stored-memory-channels = 依次扫描已存储的记忆信道
scanner-work-through-slice-band-channel = 按信道网格依次扫描频段的一个片段
scanner-stop = 停止
scanner-start = 开始
scanner-from = 起始
scanner-step = 步进
scanner-mode = 模式
scanner-range-empty-high-edge-has = 该范围为空——高端必须至少比低端高出一个信道。
scanner-skip-while-holding-pass-over = 跳过——此后它会保持跳过该信道
scanner-skipping = 跳过中
scanner-clear = 清除
scanner-stop-passing-over-any-them = 停止跳过其中任何一个
scanner-stop-passing-over-channel = 停止跳过此信道
scanner-no-memory-channels-stored-yet = 尚未存储任何记忆信道——请在 MEM 窗口中存储一些。
scanner-skip-channel-pass-over = 跳过信道以越过它
scanner-skip = 跳过
scanner-look-all-channels-fall-inside = 在全景图所用的同一变换上，查找落在同一接收机窗口内的所有信道，并且只调谐到有信号的那些。这样，一个频段上的列表无论多长，每一轮只需调谐一次，而不是每个信道都要等待稳定的时间。扫描在停止之前仍会在每个候选信道上收听，因此停止的条件不变——但扫描是通过 FFT 而非接收机的滤波器进行测量的，因此如果它开始在空无一物处停下，请检查阈值。
scanner-radio-hands-over-demodulated-audio = 此电台输出的是解调后的音频，且自身没有可供搜索的频谱，因此无论如何，它的记忆扫描都会访问每一个信道。
scanner-fast = 快速
scanner-read-list-off-spectrum-instead = 从频谱中读取列表，而不是访问每一个信道
scanner-folders = 文件夹
scanner-scan-every-folder-every-folder = 扫描每一个文件夹，以及从现在起创建的每一个文件夹。若只想扫描特定文件夹，请改为选择它们。
scanner-unfiled = 未归档
scanner-nothing-filed-under-folders-you = 你所选文件夹中没有任何归档内容。
scanner-stops = 停止条件
scanner-use-receiver-s-own-squelch = 使用接收机自己的静噪，使扫描恰好在音频会打开的地方停止
scanner-squelch-off-scan-will-stop = 静噪已关闭——扫描会在它检查的第一个内容上停止
scanner-channel-power-signal-has-reach = 信号必须达到的信道功率，才能使扫描停止
scanner-listens = 收听时间
scanner-how-long-stay-candidate-before = 在判断候选信道之前在其上停留多久。低于约十分之一秒时，电平表尚未稳定，弱信号会被漏掉
scanner-resumes = 恢复
scanner-carry-once-signal-drops = 信号消失后继续
scanner-carry-after-fixed-time-regardless = 无论如何，经过固定时间后继续
scanner-stay-until-you-press-next = 停留直到你按下“下一个”
scanner-how-long-stay = 停留时长
scanner-how-long-wait-after-signal = 信号消失后等待多久
scanner-move-don-t-stop-channel = 继续前进，并且不再在此信道上停止
scanner-next = 下一个
scanner-move-now = 立即继续

## ADS-B（src/app/panels/adsb.rs）

adsb-starting-ads-b-decoder = 正在启动 ADS-B 解码器……
adsb-ads-b = ADS-B
adsb-t-1090-mhz-mode-s = 1090 MHz Mode S
adsb-tune-receiver-1090-000-mhz = 将接收机调谐到 1090.000 MHz，即全球通用的 ADS-B 下行频率。只有一个信道。
adsb-setup = 设置
adsb-timeouts-trail-length-how-far = 超时、轨迹长度，以及速度矢量延伸的距离
adsb-aircraft = 飞机
adsb-nothing-heard-yet = 尚未听到任何内容
adsb-ground = 在地面
adsb-position-stale = 位置已过时
adsb-gnss-altitude = GNSS 高度
adsb-ground-speed = 地速
adsb-vertical-rate = 垂直速率
adsb-first-heard = 首次收听
adsb-center = 居中
adsb-put-aircraft-middle-map = 将此飞机置于地图中央
adsb-last-frame-accepted-aircraft = 从此飞机接受的最后一帧
adsb-ads-b-setup = ADS-B 设置
adsb-drop-map-after = 超过以下时间后从地图上移除
adsb-without-position-report = 没有位置报告
adsb-past-aircraft-comes-off-map = 超过此时间后，飞机会从地图上移除，其行会变灰。它不是淡出：位置过时的暗淡方块仍然是关于飞机位置的一种声明，与真实的符号使用相同的墨色。
adsb-drop-list-after = 超过以下时间后从列表中移除
adsb-nothing-heard-all = 完全没有听到任何内容
adsb-trail-length = 轨迹长度
adsb-points = {" "}个点
adsb-history-dots-behind-each-target = 每个目标身后的历史轨迹点
adsb-speed-vector = 速度矢量
adsb-how-far-ahead-leader-line = 引导线延伸的距离
adsb-one-minute-usual-radar-convention = 一分钟是雷达的惯例：该线段的长度恰好等于飞机在该时间内飞行的距离，因此在任何缩放比例下，两条等长的引导线都代表相同的速度。设为零则关闭它们。
adsb-track-most = 最多跟踪
adsb-aircraft-2 = {" "}架飞机
adsb-aircraft-ground-are-placed-against = 在地面上的飞机以电台自身的位置为基准进行放置——表面应答器本身没有明确的解码结果——因此，若附近的机场没有显示任何内容，请在数字模式设置中填写“我的网格”。
adsb-call = 呼号
adsb-icao = ICAO
adsb-v-s = 垂直速率

## DRM（src/app/drm.rs）

drm-waiting-receiver = 正在等待接收机……
drm-not-drm-set-mode-drm = 不在 DRM 模式。请在数字短波广播上将模式设为 DRM——表盘应位于信道中心，而不是边带上。
drm-no-drm-signal-locked-decoder = 尚未锁定 DRM 信号。在干净的载波上，解码器需要几秒钟：DRM 在能够读取任何内容之前，会进行 400 ms 或 2 s 的交织。
drm-station-s-audio-codec-not = 此电台的音频编解码器在此无法解码。xHE-AAC 需要系统中的 libfdk-aac——它无法内置，因为其许可证与本程序的许可证不兼容。请安装它（Debian/Ubuntu：libfdk-aac2，Arch：libfdk-aac，macOS：brew install fdk-aac，Windows：与 sdroxide.exe 放在同一目录中的 libfdk-aac-2.dll），然后重新启动。此屏幕上的其他内容均可正常解码。
drm-quality = 质量
drm-last-minute = 最近一分钟
drm-waiting-locked-signal = 等待锁定的信号
drm-audio-frames-decoding-but-station = 音频帧正在解码——但此电台的编解码器在此无法解码
drm-audio-frames-decoding = 音频帧正在解码
drm-samples-reaching-decoder = 采样正在到达解码器
drm-time = 时间
drm-symbol-timing-recovered = 符号时序已恢复
drm-frame = 帧
drm-transmission-frames-found = 已找到的传输帧
drm-fast-access-channel-what-transmission = 快速访问信道——即该传输的内容
drm-service-description-channel-what-services = 服务描述信道——即这些服务是什么
drm-audio = 音频
drm-constellation = 星座图
drm-waiting-decoded-symbols = 等待解码出的符号
drm-sampled-across-frame = 在整个帧上采样
drm-whole-frame = 整帧
drm-mode = 模式
drm-robustness-mode-channel-width-ground = 稳健模式和信道宽度。A 用于地波路径，承载量最大；D 用于严重散射的天波路径，承载量最小。
drm-interleave = 交织
drm-how-far-transmission-spreads-each = 传输在时间上将每一帧分散得多远。长交织可抵御更深的衰落，但相应地获取所需的时间也更长。
drm-protection = 保护
drm-offset = 偏移
drm-residual-sample-clock-error-against = 相对于发射机的残余采样时钟误差。数值大且稳定，说明是接收机的参考有偏差，而非广播本身。
drm-doppler = 多普勒
drm-delay = 时延
drm-doppler-delay-spread-path-how = 路径的多普勒和时延扩展——即它移动得多快，以及其回波到达的时间相隔多远。
drm-service = 服务
drm-decode-service-multiplex = 解码复用中的此服务

## AtCHAT（src/app/panels/atchat.rs）

atchat-starting-atchat-net-station = 正在启动 AtCHAT NET 电台……
atchat-atchat = ATCHAT
atchat-net-multi-station-keyboard-file = NET——多电台键盘与文件
atchat-net = {" "}在网{" "}
atchat-net-click-leave-state-kept = 在网上——点击即可离开（状态会保留；再次点击即可重新加入）
atchat-rejoin = {" "}重新加入{" "}
atchat-off-net-click-rejoin = 不在网上——点击即可重新加入
atchat-off-net = 不在网上
atchat-carrier = ● 载波
atchat-another-station-transmitting-channel-busy = 另一电台正在发射——信道忙。
atchat-tx = ● 发射
atchat-setup = ⚙ 设置
atchat-station-callsign-virtual-channel-address = 电台呼号和虚拟信道地址
atchat-virtual-channel = 虚拟信道
atchat-work-net-over-tcp-endpoint = 通过 TCP 端点而非电台运行网络——一个兼容 channel_server 的回环，用于没有电台时的开发。关闭则使电台上空。
atchat-roster = 成员名单
atchat-nobody-heard-yet = 尚未听到任何电台
atchat-chat = {" "}聊天{" "}
atchat-common-channel-lines-here-go = 公共信道——此处的消息发送给所有人（ALL）
atchat-log = {" "}日志{" "}
atchat-station-s-own-air-activity = 电台自身的空中活动——主节点选举、成员名单老化、ARQ 重试
atchat-no-activity-logged-yet = 尚未记录任何活动。
atchat-no-messages-yet = 尚无消息。
atchat-message-everyone-all = 发给所有人的消息（ALL）
atchat-send = {" "}发送{" "}
atchat-transfers = 传输
atchat-all = → 全部
atchat-sent-file-follows-active-chat = 发送的文件会跟随当前活动的聊天标签页
atchat-send-file = {" "}发送文件{" "}
atchat-send-file-image-over-air = 通过空中发送文件或图片，采用分块 CRC-ARQ
atchat-nothing-flight = 没有正在传输的内容
atchat-images = 图片
atchat-no-images-received = 尚未收到图片。


## RDS（src/app/rds.rs）

rds-station = 电台
rds-diagnostics = 诊断
rds-which-programme-type-table-read = 依据哪一张节目类型表来解读此电台。RDS 用于北美以外的所有地区；RBDS 则增加了美国电台身份码所拼写出的呼号。“自动”会跟随电台发送的扩展国家代码；在收到它之前，它不会拼写出呼号——因为它所读取的身份范围与九个国家的 RDS 代码共用，因此若电台不发送国家代码，请选择 RBDS 来为其命名。
rds-waiting-receiver = 正在等待接收机……
rds-no-rds-station-not-every = 此电台没有 RDS。并非每座发射机都带有它，较弱的发射机可能携带了它，却未能收到。
rds-rds-only-carried-wfm-broadcast = RDS 仅由调频广播电台携带。
rds-holding = 保持
rds-block-sync-has-dropped-last = 数据块同步已丢失——这是最后一次读取的电台。
rds-identity = 身份
rds-programme = 节目
rds-described = 描述为
rds-content = 内容
rds-music = 音乐
rds-speech = 语音
rds-traffic = 交通
rds-announcement-air-now = 当前正在播出的公告
rds-carried-station = 由此电台携带
rds-not-carried = 未携带
rds-radio-text = 电台文本
rds-station-clock = 电台时钟
rds-country-code = 国家代码
rds-also = 同时在
rds-also-tagged = 同时标记为
rds-sync = 同步
rds-no-sync = 无同步
rds-blocks-failed-their-check-plus = 未通过校验的数据块，以及纠错器不得不修复的数据块。纠正也算作错误，这是有意为之：数字上升是电台即将停止解码的第一个迹象。
rds-front-end-overloading = 前端过载
rds-converter-clipping-which-puts-distortion = 变换器正在削波，这会使失真遍布整个复合信号——包括 57 kHz 数据副载波。请调低射频增益，或接入衰减器：RDS 在任何内容可以听到之前大约就会受损一分贝，因此听起来完美的电台，其强度仍可能远超出其数据能够承受的程度。
rds-group-types = 组类型
rds-recent-groups-newest-last = 最近的组——最新的在最后
rds-nothing-decoded-yet = 尚无任何解码内容

## Winlink（src/app/winlink.rs）

winlink-mail = 邮件
winlink-abort = 中止
winlink-stop-session-progress = 停止正在进行的会话
winlink-connect = 连接
winlink-compose = 撰写
winlink-connecting = 正在连接……
winlink-not-connected-yet = 尚未连接
winlink-loading = 加载中……
winlink-date = 日期
winlink-from = 发件人
winlink-subject = 主题
winlink-no-subject = （无主题）
winlink-delete-message = 删除此消息
winlink-arch = 存档
winlink-move-archive = 移至存档
winlink-select-message = 选择一条消息
winlink-reply = 回复
winlink-delete = 删除
winlink-to = 收件人
winlink-cc = 抄送
winlink-subject-2 = 主题
winlink-callsign-smtp-someone-example-org = 一个呼号，或用于互联网邮件的 SMTP:someone@example.org。多个之间用逗号分隔。
winlink-file-outbox = 存入发件箱
winlink-discard = 丢弃
winlink-filed-messages-go-out-next = 已存入的消息将在下一次会话中发出
winlink-mail-log = 邮件日志
winlink-session-running = 会话运行中……
winlink-no-session-yet = 尚无会话

## WSPR（src/app/panels/wspr.rs）

wspr-receptions = 接收记录
wspr-save-wspr-reception-list-csv = 将 WSPR 接收列表保存为 CSV
wspr-wspr = WSPR
wspr-weak-signal-propagation-beacon = 弱信号传播信标
wspr-time-left-two-minute-slot = 此两分钟时隙的剩余时间。时隙结束时，录音会交给解码器，下一次发射决定随即生效。
wspr-tx = ● 发射
wspr-decoding = 解码中……
wspr-band = 频段
wspr-last-slot = 上一时隙
wspr-nothing-heard = 未听到任何内容
wspr-receive-only = 仅接收
wspr-next-slot = 下一时隙
wspr-waiting-radio-take-change = 正在等待电台接受更改……
wspr-hop-next = 下一跳
wspr-transmit = 发射
wspr-receive-only-2 = 仅接收。
wspr-power = 功率
wspr-what-you-actually-radiate = 你实际辐射的功率
wspr-goes-out-message-everyone-who = 此数值会包含在消息中，每个收听到你的人都会据此判断路径——因此这里若给出乐观的数值，他们的测量结果和你的都会出错。
wspr-roam = 漫游
wspr-pick-different-offset-inside-200 = 每次发射都在 200 Hz 窗口内选择不同的偏移。这是惯例：200 赫兹由所有人共用，只有在没有人停在其中央时才行得通。关闭则保持在光标所在的位置。
wspr-band-hop = 跨频段跳转
wspr-move-dial-band-band-between = 在时隙之间于各频段之间移动表盘，使一台接收机采样整个频谱，而不是其中的一个片段。若你自己转动 VFO，跳转会暂停——再次按下此项即可恢复。
wspr-upload = 上传
wspr-send-what-station-decodes-wsprnet = 将此电台解码的内容发送到 wsprnet.org。报告你所听到的内容，正是使 WSPR 接收机成为网络一部分的原因；它不会向空中发射任何内容。
wspr-who-heard-me = 谁收听到了我
wspr-ask-wsprnet-org-every-few = 每隔几分钟询问 wsprnet.org 哪些电台解码了此电台。WSPR 没有任何形式的确认，因此这是信标了解自身覆盖范围的唯一途径。
wspr-bands = 频段
wspr-none-selected-hopping-has-nowhere = 未选择任何频段——跳转无处可去
wspr-set-your-callsign-grid-general = 发射前，请在“设置”的“常规”选项卡中设置你的呼号和网格。

## 分组（src/app/panels/packet.rs）

packet-monitor = 监视
packet-busy = 忙
packet-frames-arrived-but-failed-their = 已到达但未通过校验序列的帧——可能是碰撞、衰落，或信号太弱而无法读取。
packet-setup = ⚙ 设置
packet-station-callsign-speed-tx-delay = 电台呼号、速率、发射延迟、数字中继路径、信标和 KISS 服务器
packet-starting-packet-modem = 正在启动分组调制解调器……
packet-nothing-heard-yet = 尚未听到任何内容
packet-terminal = 终端
packet-connected = {" "}已连接{" "}
packet-calling = {" "}呼叫中{" "}
packet-idle = {" "}空闲{" "}
packet-extended-sequence-numbers-window-up = 扩展序列号——窗口最多可达 127 帧。
packet-frames-sent-not-yet-acknowledged = 已发送但尚未被对方确认的帧。
packet-retries-against-link-s-limit = 针对此链路上限的重试次数。当未确认的帧保持不动而计数不断上升时，从此端看来，这就是衰落路径的样子。
packet-mail-window-using-link = 邮件窗口正在使用此链路。
packet-one-radio-one-channel-one = 一台电台、一个信道、一条链路。Winlink 会话和此终端是使用它的两种方式，谁先请求，谁就能使用。
packet-digipeaters-nearest-first-separated-commas = 数字中继，最近的在前，以逗号分隔——OE3XLR-1,OE3XMS-1。对于你能直接收听到的电台，或要使用分组设置中的默认路径，请将其留空。
packet-disconnect = {" "}断开{" "}
packet-hang-up-properly-wait-far = 正常挂断，并等待对方确认。改为切换模式则会结束会话，而不会告知任何人。
packet-connect = {" "}连接{" "}
packet-mail-window-has-link-finish = 邮件窗口占用着此链路。请先完成或停止该会话。
packet-station-has-no-callsign-yet = 此电台尚无呼号——请在监视面板上方的“设置”中设置一个。在设置之前，不会发射任何内容。
packet-call-station-connected-mode-node = 以连接模式呼叫此电台——节点、BBS 或另一位操作员。
packet-needs-callsign-call = 需要一个要呼叫的呼号。
packet-clear = {" "}清除{" "}
packet-empty-transcript-link-untouched-connected = 清空记录。链路不受影响——已连接的电台保持连接。
packet-not-connected-type-callsign-above = 未连接。请在上方输入一个呼号，然后按“连接”以呼叫节点或 BBS。
packet-type-here = 在此输入
packet-not-connected = 未连接
packet-send = {" "}发送{" "}
packet-send-line-node-bbs-reads = 发送此行。节点或 BBS 每次读取一行。

## FSQ（src/app/panels/fsq.rs）

fsq-speed = 速率
fsq-call = 呼叫
fsq-callsign-directed-fsqcall-messages-defaults = 用于定向（FSQCALL）消息的呼号；默认为你的呼号
fsq-tx = ● 发射
fsq-contacts = 通联
fsq-heard = 收听
fsq-none = — 无 —
fsq-allcall = ALLCALL
fsq-image = 图片
fsq-clear = 清除
fsq-forget-pictures-received-so-far = 忘记迄今为止接收到的图片
fsq-send-image = 发送图片……
fsq-right-click-remove = 右键点击即可移除
fsq-remove-picture = 移除此图片
fsq-listening = — 收听中 —
fsq-message = 消息……
fsq-send = {" "}发送{" "}
fsq-call-cq = {" "}呼叫 CQ{" "}
fsq-heard-2 = {" "}？收听{" "}
fsq-clear-2 = {" "}清除{" "}
fsq-fsq-contacts = FSQ 通联
fsq-add = 添加

## 公共 SDR（src/app/publicsdr.rs）

publicsdr-not-stated = 未说明
publicsdr-public-sdrs = 公共 SDR
publicsdr-available = 可用
publicsdr-hide-receivers-are-full-ones = 隐藏已满的接收机，以及其操作员未向浏览器以外的应用开放任何通道的接收机
publicsdr-band = 在频段内
publicsdr-only-receivers-cover-current-dial = 仅显示覆盖当前表盘频率的接收机
publicsdr-low-bw = 低带宽
publicsdr-map-cities = 城市
publicsdr-map-cities-tip = 在此地图上显示或隐藏城市。
publicsdr-view-list = 列表
publicsdr-view-map = 地图
publicsdr-view-map-tip = 在世界地图上显示同样的接收机。点击一个点即可选择其下的所有接收机。
publicsdr-map-pick = 点击一个点以选择其下的接收机。
publicsdr-map-ghost = { $name }：当前不在目录中 — 上次在此出现于 { $seen }
publicsdr-map-names = 名称
publicsdr-map-names-tip = 在地图上显示接收机名称。关闭：只显示所选接收机的名称。
publicsdr-check-online = 在线
publicsdr-check-silent = 无响应
publicsdr-check-waiting = 检查中…
publicsdr-check-no-answer = 20 秒内无响应。
publicsdr-take-spyserver-low-bandwidth-shape = 以低带宽形式接入 SpyServer：一个跟随表盘的窄幅 I/Q 窗口，加上服务器自己的频段视图，而不是数兆比特的宽带 I/Q。对 KiwiSDR 无影响，因为它只有这一种形式。
publicsdr-refresh = ⟳ 刷新
publicsdr-fetch-both-directories-again = 重新获取两个目录
publicsdr-name-place-antenna-band = 名称、地点、天线、频段
publicsdr-clear-search = 清除搜索
publicsdr-fetching-receiver-lists = 正在获取接收机列表……
publicsdr-machine-radio-attached-does-not = 此电台所连接的机器不应答设备查询，因此它也无法获取列表。
publicsdr-no-match = 无匹配
publicsdr-nothing-matches-try-turning-filter = 没有任何内容匹配——请尝试重新打开某个过滤标签

## HD Radio（src/app/hd.rs）

hd-hd-radio = HD Radio
hd-waiting-receiver = 正在等待接收机……
hd-not-hd-radio-set-mode = 不在 HD Radio 模式。请在携带数字边带的调频广播上将模式设为 HD Radio——表盘应位于模拟载波的中心，而不是其旁边。
hd-no-hd-radio-lock-digital = 未锁定 HD Radio。数字边带的发射功率比模拟载波低约 20 dB，因此一个可以听到的电台，仍可能噪声太大而无法解码：在干净的信号上，解码器需要几秒钟。
hd-sync = 同步
hd-ofdm-frame-timing-recovered = OFDM 帧时序已恢复
hd-audio = 音频
hd-audio-frames-decoding = 音频帧正在解码
hd-quality = 质量
hd-lower = 下边带
hd-upper = 上边带
hd-last-minute = 最近一分钟
hd-waiting-locked-signal = 等待锁定的信号
hd-mer-l = MER L
hd-modulation-error-ratio-lower-digital = 下数字边带的调制误差比。
hd-mer-u = MER U
hd-modulation-error-ratio-upper-digital = 上数字边带的调制误差比。
hd-cber = CBER
hd-channel-bit-error-ratio-after = 内码之后的信道误码率。低于约 1e-3 时，外码可以纠正它；高于此值，音频就会开始断续。
hd-offset = 偏移
hd-residual-carrier-frequency-offset-large = 残余载波频率偏移。数值大且稳定，说明是接收机的参考有偏差，而非广播本身。
hd-psmi = PSMI
hd-primary-service-mode-indicator-which = 主服务模式指示器：电台正在发射的是哪一种调频混合服务模式。
hd-programme = 节目
hd-restricted-access = {" "}——受限访问

## 频段（src/app/bands.rs）

bands-bands = 频段
bands-daylight = ☀ 白天
bands-night = ☾ 夜间
bands-no-forecast-yet-fetching-hamqsl = · 尚无预报——正在从 HAMQSL.com 获取
bands-forecast-download-off = · 预报下载已关闭（设置 → 界面）
bands-band = 频段
bands-condx = CONDX
bands-paths = 路径
bands-reach = 覆盖
bands-best = 最佳
bands-condx-global-forecast-rest-what = CONDX 是全球预报；其余则是实际听到的内容。某一行为空，意味着没有解码到任何东西——这可能是因为该频段关闭，也可能只是因为没有人在其上。
bands-ibp-beacons = IBP 信标
bands-meteor-showers = 流星雨
bands-peak = 峰值

## 信号列表（src/app/spots.rs）

spots-pota = POTA
spots-sota = SOTA
spots-freedv = FREEDV
spots-spots = 信号
spots-longwave-shortwave-broadcast-stations-air = 当前正在播出的长波和短波广播电台
spots-view = 视图内
spots-only-spots-inside-panadapter-span = 仅显示全景图跨度之内的信号
spots-setup = ⚙ 设置
spots-feeds-lookup-upload-settings = 数据源、查询和上传设置
spots-call-station-site-frequency = 呼号、电台、站点、频率
spots-clear-search = 清除搜索
spots-no-match = 无匹配
spots-no-spots-enable-feed-setup = 没有信号——请在 ⚙ 设置 中启用一个数据源
spots-nothing-matches-search = 没有任何内容匹配搜索

## ISM（src/app/ism.rs）

ism-ism-devices = ISM 设备
ism-decoding = 解码中
ism-needs-wideband-iq-source-cat = 需要宽带 IQ 信源；CAT 电台只发送音频
ism-how-far-burst-must-stand = 突发信号在被解码之前，必须高出该信道自身底噪多少
ism-give-each-band-width-normally = 为每个频段提供其通常所需的宽度——315、345 和 433 MHz 的 OOK 频段需要四分之一兆赫，868 和 915 则需要一整个兆赫。
ism-several-hundred-more-device-decoders = 另有数百个设备解码器，来自 rtl_433 项目，已内置。可读取 sdroxide 自身解码器完全无法触及的 433 MHz OOK 设备。凡是两者都能读取的设备，rtl_433 知道的变体要多得多，并会接管。
ism-t-1-your-own = 1 个你自己的
ism-device-decoders-registered-how-many = 已注册的设备解码器、其中来自你的 rtl433_flex.conf 的数量，以及自启动以来已解码的发射数。
ism-reload-decoders = 重新加载解码器
ism-re-read-rtl433-flex-conf = 编辑后重新读取 rtl433_flex.conf。已收听到的设备仍会保留在列表中。
ism-rtl-433-knows-more-variants = rtl_433 比 sdroxide 自身的解码器了解这些设备的更多变体，因此在此收听时由它读取。选择另一个频段，sdroxide 自身的解码器便会重新接管。
ism-decoder-your-rtl433-flex-conf = 你的 rtl433_flex.conf 中的一个解码器被拒绝。其余的已正常加载。请修正该行，然后按“重新加载解码器”。
ism-waiting-decoder = 正在等待解码器……
ism-where-decoder-actually-listening-follows = 解码器实际在收听的位置。它跟随接收机的硬件中心，而不是 VFO——在宽幅前端上，两者可能相距甚远，而决定哪些信道可以到达的正是这个窗口。
ism-transmissions-gate-opened-how-many = 闸门打开过的发射次数，以及其中产生有效帧的次数。突发很多而解码很少，说明频段上充斥着 sdroxide 尚无法读取的设备。完全没有，则说明没有任何东西达到阈值——请调低阈值，或检查表盘是否在该频段上。
ism-nothing-heard-yet = 尚未听到任何内容
ism-click-tune-receiver =
    {""}

    点击即可将接收机调谐到它

## 应用（src/app/mod.rs）

app-hpsdr = HPSDR
app-rtl-sdr = RTL-SDR
app-rtl-sdr-rtl-tcp = RTL-SDR（rtl_tcp）
app-spyserver-vfo = SpyServer（VFO）
app-airspy-hf = Airspy HF+
app-sdrplay = SDRplay
app-elad = ELAD
app-airspy = Airspy
app-sxceiver = SXceiver
app-usrp = USRP

## NAVTEX（src/app/panels/navtex.rs）

navtex-messages = 消息
navtex-sync = 同步
navtex-character-phase-locked-signal-being = 字符相位已锁定——正在读取信号。
navtex-no-character-phase-yet-no = 尚无字符相位：没有信号，或不是此信号。
navtex-characters-whose-first-copy-was = 首次副本已损坏、取自五个时隙之后的重复副本的字符，以及两份副本都已损坏的字符。NAVTEX 的每个字符都会发送两次；这就是它全部的纠错手段。
navtex-swap-mark-space-tones-signal = 交换标记音和空号音，适用于在另一个边带上接收到的信号。关闭即为信道频率上的上边带，这也是该服务所有已发布的调谐说明所采用的方式。
navtex-starting-navtex-receiver = 正在启动 NAVTEX 接收机……
navtex-nothing-yet-station-transmits-ten = 尚无内容——电台每四小时发射十分钟
navtex-cut-short = 被提前切断
navtex-message = 消息
navtex-time-day-stated-message-utc = 消息中所述的时刻（UTC）：仅供显示
navtex-receiving = 接收中
navtex-monitor = 监视
navtex-everything-decoded-message-not = 所有解码内容，无论是否为消息

## 射频绘图（src/app/panels/rf_paint.rs）
rfpaint-rf-paint = 射频绘图
rfpaint-scan-speed = 扫描速度
rfpaint-how-fast-text-image-scanned = 文字或图像被扫描到瀑布图上的速度。越低越慢、越清晰；100% = 基准速率，25%（中间）为默认值。
rfpaint-abort = 中止
rfpaint-text-paint = 文字绘图
rfpaint-type-text-paint = 输入要绘制的文字……
rfpaint-preview-waterfall = 预览瀑布图
rfpaint-type-text-preview = 输入文字以预览
rfpaint-transmit = {"  "}发射{"  "}
rfpaint-image-paint = 图像绘图
rfpaint-no-image-loaded = 未加载图像
rfpaint-load-image = 加载图像……
rfpaint-load-image-preview = 加载一张图像以预览

## HFDL（src/app/hfdl.rs）

hfdl-listen = 收听
hfdl-switches-engine-s-hfdl-downconverter = 开启或关闭引擎的 HFDL 下变频器和解码器。无论窗口是否打开，解码都会占用一个 24 kHz 的通道和一个工作线程。
hfdl-channel-assigned-frequency-listen-anything = 要收听的信道（指定频率）。频段内的任何频率都可以——HFDL 电台在 2.8 至 22 MHz 发射——但会对准这里居中的固定 24 kHz 通道。
hfdl-save-hfdl-decode-log-file = 将 HFDL 解码日志保存到文件
hfdl-riverhead-northern-atlantic-north-america = Riverhead（北大西洋 / 北美）——默认值，也是解码器验证时所用的信道。
hfdl-hfdl-assigned-frequency = 一个 HFDL 指定频率。
hfdl-one-your-saved-channels-click = 你已保存的信道之一——点击即可调谐，右键点击即可移除。
hfdl-hfdl-selected-but-decoding-off = 已选择 HFDL，但解码已关闭——请按上方的“收听”
hfdl-decodes = 解码
hfdl-nothing-decoded-yet-ground-station = 尚无解码内容——一旦信道中出现地面站，其信标每约 32 秒重复一次。
hfdl-no-decode-matches-filter = 没有解码内容符合过滤条件。
hfdl-decoding = 解码中
hfdl-decoding-off = 解码已关闭
hfdl-dbfs = — dBFS

## RADE（src/app/panels/rade.rs）

rade-rade = RADE
rade-freedv-v1-digital-voice = FreeDV V1 数字话音
rade-tx = ● 发射
rade-mute-analog = 静音模拟信号
rade-mute-demodulated-audio-so-only = 静音解调音频，使只能听到解码出的语音。在其他情况下，每当调制解调器没有可播放的内容时，原始信号都会被直接传出——正是这种嘶嘶声让你能在同步之前找到一次发射，因此调谐时请保持此项关闭。
rade-sync = 同步
rade-how-far-received-signal-sits = 接收到的信号与调制解调器所期望的位置相差多远。数值大的仍能解码——捕获环路会跟踪它们——但稍微移动表盘使其接近零，可获得最佳余量。
rade-snr = 信噪比 —
rade-callsign-station-put-end-over =
    电台在其通话结束（End-of-Over）帧中放入的呼号——这是 RADE 通话中唯一发送它的时机。它也是报告给 FreeDV Reporter 的内容，因此你所收听到的电台会在 qso.freedv.org 上显示为由你收听到。

    信号消失后保持一分钟，并在你开始自己的通话时清除：它指的是当前频率上的电台，而不是接收机曾经听到过的所有电台。
rade-stop-talking = 停止说话
rade-talk-ptt = 说话（PTT）
rade-open-close-rade-over-modem = 开始或结束一次 RADE 通话。调制解调器在第一帧发出之前需要约 120 ms 的语音，并在你停止时发送一个通话结束帧，因此发射会在按钮松开后再持续一小段时间。
rade-rade-v1-occupies-roughly-1060 = RADE V1 占用通带中约 1060–1880 Hz——上边带，或在 160、80 和 40 米上为下边带，RADE 在这些频段上保留了电话练习，sdroxide 会为你切换。将信号置于瀑布图上的阴影频带之内；调制解调器便会从那里找到它。

## 框架（src/app/frame.rs）

frame-acknowledge = 确认
frame-dismiss = 关闭
frame-get-sdroxide-com = 在 sdroxide.com 获取
frame-reconnecting = 正在重新连接……
frame-not-connected = 未连接。
frame-reconnect-now = 立即重新连接
frame-transmit-lockout-disabled = ⚠ 发射锁定已禁用
frame-engine-was-started-oob-tx = 此引擎是以 --oob-tx 启动的。业余频段锁定已关闭：它将在硬件支持的任何频率上键控发射机。
frame-transmitting-outside-your-licence-offence = 在执照范围之外发射，在每个颁发执照的国家都属于违法行为。只有当你获得授权使用即将键控的频率时才应继续——例如 MARS/CAP 或商业执照、实验许可证，或假负载。
frame-i-understand = {"  "}我已了解{"  "}
frame-restart-without-oob-tx-put = 不带 --oob-tx 重新启动，以恢复锁定。

## S 表（src/widgets/smeter.rs）

smeter-alc-how-hard-transmitter-being = ALC——发射机被驱动的力度，以其最大输出的百分比表示。100% 是满量程，而不是以瓦计的满功率：它在天线上相当于多少，取决于电台和驱动设置。
smeter-radio-reports-no-forward-power =
    {""}

    此电台不报告正向功率，因此 ALC 旁边的数值仍是 ALC 读数，而非瓦数。
smeter-no-swr-radio-has-no =
    {""}

    没有驻波比：此电台没有用于测量它的电桥，sdroxide 不会凭空制造一个——它只能显示电台自身所报告的内容。带有驻波比表的电台（通过 CAT、TCI 或局域网链路）以及 HPSDR 板卡都会在此处填入数值；普通的 SDR 发射机则不能。
smeter-no-signal-report-yet-receiver = 尚无信号报告——接收机尚未提供表头读数。
smeter-s9-73-dbm-each-s =
    {""}

    S9 为 −73 dBm，其下每个 S 单位为 6 dB。dBm 数值的可靠程度仅取决于前端的校准——未校准的接收机会以 dBFS 报告，却标注为 dBm。
smeter-ovld-converter-into-rails-so =
    {""}

    OVLD：变换器已达到其极限，因此所显示的电平低估了信号。请调低前端增益。
smeter-table-held = ，表格已保持
smeter-ps = PS --

## 多电台（src/multi.rs）

multi-tx = ● 发射
multi-mute-radio-s-audio = 静音此电台的音频
multi-show-radio-here = 在此处显示此电台
multi-show-on-map = 在地图上显示
multi-add-radio-computer-settings-radio = 在本计算机上添加电台（若要在电台站点上添加，请使用“设置 → 电台”）
multi-add-radio = 添加电台
multi-take-radio-off-row = 把这台电台移出这一行。它继续运行；用圆环关闭它。
multi-open-pane = 用一台未在屏幕上的电台打开另一个窗格
multi-pick-radio = 电台 ▼
multi-pick-radio-tip = 把一台电台放进这一行。圆环标出连接已打开的电台。
multi-new-radio = 新电台…
multi-asked-station-another-radio = 已向电台站点请求另一台电台……
multi-client-cannot-open-connection = 此客户端无法打开连接。

## 网络（src/app/net.rs）

net-fail = 失败
net-will-retry = {" "}（将重试）
net-full-resync-downloading = 完全重新同步：正在下载……
net-downloading = 正在下载……
net-full-resync-log-already-matches = 完全重新同步：日志已与 Wavelog 一致
net-full-resync-review-changes-below = 完全重新同步：请检查下面的更改
net-full-resync-discarded-nothing-changed = 已放弃完全重新同步；没有任何更改
net-adif-import-failed-file-could = ADIF 导入失败：文件无法解析
net-channel-import-failed-file-could = 信道导入失败：文件无法读取
net-channel-import-nothing-read-chirp = 信道导入：没有可读取的内容——CHIRP CSV 文件以一行表头开始，表头列出其列名，其中必须包含“Frequency”

## 组件（src/app/panels/widgets.rs）

widgets-free-text = 自由文本
widgets-hashed-callsign-not-yet-heard = 哈希呼号，尚未完整听到
widgets-entity-unknown = 实体未知
widgets-band = 此频段
widgets-new-entity-never-worked-any = 新实体——从未通联过，任何频段均无
widgets-new-grid-square = 新网格方格
widgets-not-worked-before = 此前未通联
widgets-worked-before-but-not-band = 此前已通联，但不在此频段
widgets-call-queue = 在呼叫队列中
widgets-image = 图像

## PI4（src/app/panels/pi4.rs）

pi4-receptions = 接收记录
pi4-save-pi4-reception-list-csv = 将 PI4 接收列表保存为 CSV
pi4-next-generation-beacon = 下一代信标
pi4-time-left-one-minute-beacon = 此一分钟信标周期的剩余时间。在窗口有足够时间填充到其结束点之后，才会搜索 PI4 消息。
pi4-decoding = 解码中……
pi4-band = 频段
pi4-last-cycle = 上一周期
pi4-nothing-heard = 未听到任何内容
pi4-receive-only-tune-so-beacon = 仅接收。请调谐，使信标的 CW 识别位于 800 Hz 音频处——即该网络自身的惯例——并使四个 PI4 音调落在此解码器默认搜索它们的位置。

## 奖项（src/app/awards.rs）

awards-awards = 奖项
awards-band = 频段
awards-all = 全部
awards-dxcc = DXCC
awards-grids = 网格
awards-worked-all-states = 通联全部州
awards-cq-zones-waz = CQ 区（WAZ）
awards-dxcc-entities = DXCC 实体

## ACARS（src/app/panels/acars.rs）

acars-acars = ACARS
acars-audio-decoder-s-passband = 解码器通带内的音频。
acars-blocks-framed-blocks-whose-check = 已成帧的数据块，以及校验序列失败的数据块。
acars-starting-acars-receiver = 正在启动 ACARS 接收机……
acars-channel = 信道
acars-tune-dial-here = 在此调谐表盘。
acars-nothing-yet-acars-quiet-between = 尚无内容。ACARS 在飞机之间保持安静——将其留在一个信道上，等待突发信号。
acars-check-failed = 校验失败
acars-block-check-sequence-did-not = 数据块校验序列不匹配——文字可能有误。
acars-no-text = （无文字）

## 登录（src/login.rs）

login-sdr-oxide = SDR OXIDE
login-username = 用户名
login-password = 密码
login-remember-device = 在此设备上记住
login-signs-without-asking-next-time = 下次无需询问即可登录，并且 3D 视图的选项卡也无需询问即可进入。密码以明文保存，因此在他人也会使用的机器上请关闭此项。
login-keeps-password-device-clear = 将密码以明文保存在此设备上。
login-remember-station = 记住此电台
login-remember-me = 记住我
login-station-keeps-sign-in = 由电台而不是此页面保存你的登录——其他电台无需再填写，离开时也无需清除任何内容。
login-not-this-time = 本次不记住：浏览器关闭时会忘记，因此下次会话会再次询问，设备上不会留下此次登录的任何信息。
login-checking = {" "}正在检查……{" "}
login-sign = {" "}登录{" "}
login-wrong-password-shuts-server-s = 密码错误会使服务器的大门关闭数秒，因此可能需要一点时间。

## 帮助（src/help.rs）

help-find = 查找
help-find-manual = 在手册中查找
help-previous-match-shift-enter-shift = 上一个匹配项（Shift+Enter、Shift+F3）
help-next-match-enter-f3 = 下一个匹配项（Enter、F3）
help-no-matches = 无匹配
help-clear-search-esc = 清除搜索（Esc）
help-sdroxide-manual = SDROXIDE 手册
help-contents = 目录

## AIS 地图（src/ais_map.rs）

aismap-t-1-station-heard-waiting-report = 已听到 1 个电台——正在等待其报告位置
aismap-double-click-reframe = 双击重新取景
aismap-zoom-out-wheel-pinch-do = 缩小——滚轮或双指捏合同样可以
aismap-zoom-about-middle-chart = 以海图中央为中心放大
aismap-frame-everything-being-tracked-again = 重新框取所有正在跟踪的内容——双击也可以

## 频谱视图（src/widgets/spectrum_view.rs）

specview-waiting-spectrum = 等待频谱……
specview-hounds = HOUNDS

## 频谱（src/app/spectrum.rs）

spectrum-window-has-no-gpu-renderer = 此窗口没有 GPU 渲染器，因此瀑布图保持标准宽度。
spectrum-machine-drawing-without-gpu-every = 此机器在没有 GPU 的情况下绘图——瀑布图的每一列都由处理器完成，而处理器已经与电台共享。
spectrum-window-renders-through-opengl-which = 此窗口通过 OpenGL 渲染，这是 sdroxide 的兼容路径——例如 Raspberry Pi、较旧的显卡，或不支持 WebGPU 的浏览器。在那里，更宽的瀑布图不值得牺牲帧率。
spectrum-wider-than-renderer-can-draw = 比此渲染器所能绘制的更宽。

## 宏（src/app/panels/macros.rs）

macros-nocall = 无呼号
macros-each-button-sends-whole-text = 每个按钮都会一次性发送其全部文字。只要屏幕上没有任何东西占用键盘，F2–F10 即按下前九个。
macros-remove-button = 删除此按钮

## 网格追踪器（src/app/grid_tracker.rs）

gridtracker-grid-tracker = 网格追踪器
gridtracker-shade-squares-heard-live-decode = 为实时解码列表中听到的方格着色，而不仅是日志中的那些——即当前正在空中的内容，以青色显示在已通联方格之下。
gridtracker-drag-pan-wheel-zoom-amber = 拖动以平移，滚轮以缩放。琥珀色 = 已通联，绿色 = 已确认，青色 = 已收听。

## 远程（src/remote.rs）

remote-server-busy-another-client-connected = 服务器忙——另一个客户端已连接
remote-transmitting-but-client-s-microphone = 正在发射，但此客户端的话筒完全没有产生任何音频——因此没有任何东西被调制。在浏览器中，请检查该页面是否获得了话筒权限，以及是否有其他应用占用着它。
remote-connection-closed = 连接已关闭

## 太阳系（src/solar3d/mod.rs）

solarwin-sdroxide-solar-system = sdroxide — 太阳系

## 太阳系按钮（src/app/solar.rs）

solarbtn-solar-system-3d-view-sun = 太阳系三维视图——太阳、地球、月球、太阳黑子和日冕物质抛射（独立窗口）
solarbtn-solar-system-3d-view-sun-2 = 太阳系三维视图——太阳、地球、月球、太阳黑子和日冕物质抛射（新的浏览器标签页）

## 宽频谱（src/widgets/wide_spectrum.rs）

## 语音（src/app/speech.rs）

speech-loading-voice = 正在加载语音……
speech-browser-client-cannot-speak-yet = 浏览器客户端尚无法发声

## 频率显示（src/widgets/freq_display.rs）

freqdisplay-double-click-type-frequency = 双击以输入频率
freqdisplay-tap-type-frequency = 点按以输入频率

## 太阳系状态（src/solar3d/state.rs）

solarstate-home = 首页
solarstate-dwarf-planets = 矮行星

## ADS-B 地图（src/adsb_map.rs）

adsbmap-double-click-reframe = 双击重新取景

## APRS 地图（src/aprs_map.rs）

aprsmap-double-click-reframe = 双击重新取景

## 太阳系网络（src/solar3d/net.rs）

solarnet-connection-closed = 连接已关闭

## 警报（src/app/alerts.rs）

alerts-could-not-start-alert-thread = 无法启动警报线程

## HFDL 地图（src/hfdl_map.rs）

hfdlmap-no-aircraft-located-yet = 尚未定位到任何飞机

## 世界地图（src/widgets/worldmap.rs）

worldmap-double-click-reframe = 双击重新取景

## 外观（src/chrome.rs）

## 双电台功率（src/app/duo_power.rs）

duopower-sets-fdm-duo-s-own = 设置 FDM-DUO 自身的功率（菜单 34 TX POWER）。驱动和调谐随后在此范围内设置功率，而 SDRoxide 每次连接时都会重新发送此设置。

## 频段状况窗口：峰值日期。

# $month 为 1–12。
bands-peak-date = { $month } 月 { $day } 日

## 下载（src/download.rs）

download-browser-could-not-read-file = 浏览器无法读取该文件

## 摩尔斯电码（src/app/morse.rs）

morse-translate = 翻译
morse-practice = 练习
morse-learn = 学习
morse-morse = 摩尔斯
morse-text-morse = 文字 → 摩尔斯
morse-morse-text = 摩尔斯 → 文字
morse-per-character-space-between-characters = 每个字符用 `.` 和 `-` 表示，字符之间有一个空格，单词之间为 ` / `。采用国际电信联盟字母表、数字、标点以及常用的程序符号。
morse-reference-translator-available-desktop-build = 桌面版本提供参考翻译器。
morse-play-what-you-type-your = 通过你自己的扬声器播放你所键入的内容
morse-play = 播放
morse-stop = 停止
morse-every-character-learned-well-done = 所有字符都已学会——做得好。
morse-drill-needs-sound-which-desktop = 练习需要声音，桌面版本具备此功能。
morse-reveal = 显示
morse-reset = 重置
morse-replay = 重播
morse-what-did-you-hear = 你听到了什么？
morse-check = 检查
morse-press-new-hear-character = 按“新建”以听到一个字符。
morse-tone = 音调
morse-speed = 速度
morse-spacing = 间距
morse-characters-go-out-speed-above = 字符以上方的速度发出；而*间隔*则以这一速度发送，这样初学者在能够以那么快的速度发送之前，就能先把字母作为一个整体听到。
morse-playback-available-desktop-build = 桌面版本提供回放功能。

## ADS-B 面板：飞机详细信息行。

adsb-row-altitude = 高度
adsb-row-track = 航迹
adsb-row-turning = 转弯
adsb-row-squawk = 应答码
adsb-row-range = 距离
adsb-row-position = 位置
adsb-row-signal = 信号
adsb-row-frames = 帧
rade-transmitting = 发射中
decodes-odd = 奇数
decodes-new = 新
js8-new = 新

## AIS 面板：船舶详细信息行。

ais-row-status = 状态
ais-row-speed = 速度
ais-row-course = 航向
ais-row-heading = 船首向
ais-row-turning = 转向
ais-row-altitude = 高度
ais-row-size = 尺寸
ais-row-draught = 吃水
ais-row-destination = 目的地
ais-row-range = 距离
ais-row-signal = 信号
ais-row-type = 类型
ais-row-position = 位置
ais-row-messages = 消息
ism-off = 关
textmodem-frequency-shifted = 频移
winlink-to-heading = 收件人

## adsbmap：带数值的消息（src/adsb_map.rs）

adsbmap-ft-min =
    {""}
    { $vr } 英尺/分钟

## aismap：带数值的消息（src/ais_map.rs）

aismap-heard-no-position-newer-than = 已听到 { $heard } 个——没有比 { $drop_map_s } 秒更新的位置
aismap-stations-heard-waiting-position-report = 已听到 { $heard } 个电台——正在等待位置报告
aismap-km-across = { $lon_span }° · 横跨约 { $km } km

## alerts：带数值的消息（src/app/alerts.rs）

alerts-alarming = 正在通过 { $device } 发出警报
alerts-alarm-device-unavailable = 警报设备不可用：{ $error }

## awards：带数值的消息（src/app/awards.rs）

awards-worked = 已通联 { $w }
awards-confirmed = 已确认 { $value }

## bands：带数值的消息（src/app/bands.rs）

bands-forecast-hamqsl-com-old = · 预报来自 HAMQSL.com，已过 { $value }
bands-ncdxf-iaru-next-beacon-s = NCDXF/IARU · 下一个信标在 { $now } 秒后
bands-hearing-means-path-open-each = { $callsign } 位于 { $location }——若能听到它，说明 { $band } 路径是开放的。每个信标每 10 秒切换到下一个频段，因此此行每个时隙都会变化。
bands-radiants-placed = 已为 { $lat }°、{ $lon }° 放置辐射点
bands-zhr = ZHR { $zhr }
bands-radiant = 辐射点 { $alt_deg }° { $az_deg }
bands-radiant-down = 辐射点在下方（{ $alt_deg }°）
bands-km-s-parent-peak-rate =
    { $value }（{ $code }）——{ $velocity_kms } km/s，母体 { $parent }。在 { $peak } 左右达到峰值速率 ZHR { $zhr }。

    辐射点位于地平线之上，意味着流星余迹能够到达你；快速的流星雨会留下寿命更长的电离余迹，供 6 米和 2 米的流星散射使用，以及短暂的 10 米开放。

## drm：带数值的消息（src/app/drm.rs）

drm-not-decoding = { $channel } 未在解码
drm-symbols = { $value } 个符号 · { $frame }
drm-not-decodable = { $value }——无法解码
drm-broadcaster-s-clock-utc = 广播方时钟 { $year }-{ $month }-{ $day } { $hour }:{ $minute } UTC

## duopower：带数值的消息（src/app/duo_power.rs）

duopower-max-fdm-duo-s-full =
    最大：FDM-DUO 的满功率输出，约 8 W。在接入 HR500 时应使用的设置。

    { $base }
duopower-t-5-w-hr500-line-will =
    5 W。在接入 HR500 的情况下，此设置在大多数频段上都不会达到满输出：请使用“最大”，并通过驱动将其调低。

    { $base }
duopower-w-running-barefoot-qrp =
    { $code } W，适用于不接功放的运行或 QRP。

    { $base }

## frame：带数值的消息（src/app/frame.rs）

frame-re-enables-transmit-after-1 = 在 { $swr }:1 之后重新启用发射。请先检查天线：若故障仍然存在，下一次发射同样会被停止。
frame-sdroxide-has-been-released = SDRoxide { $version } 已发布——当前版本为 { $sdroxide_version }。
frame-reconnecting-s = { $value } 秒后重新连接……

## gridtracker：带数值的消息（src/app/grid_tracker.rs）

gridtracker-worked = 已通联 { $worked }
gridtracker-confirmed = 已确认 { $confirmed }
gridtracker-heard = 已收听 { $heard }

## hd：带数值的消息（src/app/hd.rs）

hd-decode = 解码 { $value }（{ $codec_label }）

## hfdl：带数值的消息（src/app/hfdl.rs）

hfdl-khz-already-has-chip = { $cur_khz } kHz 已有一个芯片
hfdl-save-current-channel-khz-one = 将当前信道（{ $cur_khz } kHz）保存为你自己的一个芯片
hfdl-aircraft = { $aircraft } 架飞机
hfdl-bursts = { $bursts } 次突发
hfdl-decodes-2 = { $decodes } 次解码
hfdl-fixed = 已纠正 { $fec }

## ism：带数值的消息（src/app/ism.rs）

ism-not-decoded-yet = { $hint }——尚未解码
ism-listen-tunes-radio-there-one = 收听 { $value }——会将电台调谐到那里。一次一个频段：它们相距太远，任何接收机都无法同时覆盖两个。
ism-watch-hz-around-band-centre = 监视频段中心两侧 { $value }Hz 的范围，无论该频段原本会要求多少。较窄的范围适合无法提供该频段自身宽度的接收机；较宽的范围则能覆盖位于中心更远处的设备。你的接收机必须多提供大约三分之一的带宽。
ism-rtl-433 = rtl_433——{ $why }
ism-rtl-433-khz = rtl_433 { $value } · { $khz } kHz
ism-your-own = 你自己的 { $flex } 个
ism-decoders-decoded = { $decoders } 个解码器（{ $flex }），已解码 { $decodes }
ism-handled-rtl-433 = 由 rtl_433 处理：{ $names }
ism-flex = flex：{ $error }
ism-window-mhz-mhz-wide = 窗口 { $mhz } MHz，宽 { $e62 } MHz
ism-bursts-decoded = { $bursts } 次突发，已解码 { $decodes }
ism-devices = { $ism_reports } 个设备
ism-id-mhz-db-heard-first =
    { $protocol } 标识 { $device }
    { $e6 } MHz { $snr_db } dB 已收听 { $value } 次
    首次收听于 { $first_at } 前

## logbook：带数值的消息（src/app/logbook.rs）

logbook-confirmations-downloaded-newly-confirmed = 确认信息：已下载 { $recs } 条，新确认 { $matched } 条
logbook-qso = { $qso_log } 次通联
logbook-worked-before = ⚠ 此前已通联（{ $dupe_band }）
logbook-utc-qso = { $oldest }–{ $newest } UTC · { $value } 次通联

## app：带数值的消息（src/app/mod.rs）

app-radio = 电台 { $id }

## morse：带数值的消息（src/app/morse.rs）

morse-could-not-start-morse-audio = 无法启动摩尔斯音频线程：{ $error }
morse-no-audio-output = 没有音频输出：{ $error }
morse-about-s = 约 { $farnsworth_wpm } 秒
morse-characters-play = 正在练习的字符：{ $set }
morse-score-run = 得分：{ $correct } / { $total }。连续正确：{ $streak }（共 { $value }）。
morse-correct = 正确——{ $value }
morse-was = 那是 { $value }

## net：带数值的消息（src/app/net.rs）

net-synced-z-new-already-log = 已同步 { $when }Z：新增 { $added } 条，{ $matched } 条已在日志中
net-wavelog-2 = Wavelog：{ $line }
net-full-resync-stopped-nothing-changed = 完全重新同步已停止，没有任何更改：{ $error }
net-full-resync-applied-added-updated = 已应用完全重新同步：新增 { $add } 条，更新 { $update } 条，删除 { $remove } 条，其中 { $mark } 条你自己的通联已标记为在 Wavelog 中
net-wavelog-3 = Wavelog：{ $line }
net-adif-import-failed = ADIF 导入失败：{ $error }
net-not-unicode-read = {" "}（非 Unicode；按 { $enc } 读取）
net-received-reports-swl-left-out = ，已排除 { $n } 条接收报告（SWL）
net-adif-import-added-duplicates-skipped = ADIF 导入：新增 { $added } 条，跳过 { $skipped } 条重复项{ $reports }{ $assumed }
net-channel-import-failed = 信道导入失败：{ $error }
net-line-s-skipped = ，已跳过 { $n } 行
net-not-unicode-read-2 = {" "}（非 Unicode；按 { $enc } 读取）
net-channel-import-channel-s-read = 信道导入：已读取 { $n } 个信道{ $skipped }{ $assumed }

## acars：带数值的消息（src/app/panels/acars.rs）

acars-frames-bad = { $frames } 帧 · { $bad } 个错误
acars-mode = 模式 { $mode }

## adsb：带数值的消息（src/app/panels/adsb.rs）

adsb-aircraft-3 = { $aircraft } 架飞机
adsb-frames = { $frames } 帧
adsb-preambles = { $preambles } 个前导码
adsb-bad-crc = { $bad_crc } 个 CRC 错误
adsb-tune = 调谐 { $mhz }
adsb-ft-barometric = { $ft } 英尺（气压高度）

## ais：带数值的消息（src/app/panels/ais.rs）

ais-vessels-3 = { $vessels } 艘船舶
ais-messages = { $messages } 条消息
ais-slots = { $bursts } 个时隙
ais-bad-fcs = { $bad_fcs } 个 FCS 错误
ais-ais-mhz = AIS { $value }（{ $mhz } MHz）——{ $value2 }
ais-ais-mhz-slots-messages-floor = AIS { $value }（{ $mhz } MHz）——{ $bursts } 个时隙，{ $messages } 条消息，底噪 { $floor_dbfs } dBFS
ais-ais = AIS { $value }
ais-tune = 调谐 { $mhz }
ais-ships-heard-are-khz-off = 所听到的船舶偏离频率 { $khz } kHz（162 MHz 下为 { $mhz } ppm）——请设置前端的频率校正，否则超过约 5 kHz 的发射将无法解码
ais-over-ground = 对地 { $value }°
ais-dbfs-db-snr = { $rssi_dbfs } dBFS，信噪比 { $snr_db } dB
ais-last-type-ais = { $messages }（最后类型 { $last_type }，AIS { $channel }）
ais-ais-2 = AIS { $ch }
ais-mhz-marine-channel-ship-alternates = { $mhz } MHz——海事信道 { $ch }。船舶在两者之间交替发射，因此关闭其中一个会使每艘船被听到的频率减半。

## aprs：带数值的消息（src/app/panels/aprs.rs）

aprs-object = 来自 { $reported_by } 的对象
aprs-via = 经由 { $via }
aprs-object-reported = 对象由 { $reported_by } 报告
aprs-digit-s-blanked = ± { $ambiguity } 位数字被隐去
aprs-wind-m-s = 风 { $value }° { $value2 } m/s
aprs-gust = 阵风 { $value }
aprs-rain-mm-h = 降雨 { $value } mm/h
aprs-frame-s-last-ago = { $packets } 帧，最近一帧距今 { $last_heard }
aprs-via-2 = 经由 { $via }
aprs-bad = { $bad_frames } 个错误
aprs-other = { $non_aprs } 个其他
aprs-sent = 已发送 ×{ $tries }
aprs-stn = { $stations } 个电台
aprs-queued = { $tx_queue } 个排队中

## AtCHAT：带数值的消息（src/app/panels/atchat.rs）

atchat-master = 主节点 { $value }
atchat-last-heard-s-ago-click = { $status }——最近一次收听于 { $age_s } 秒前 · 点击打开一个私信标签页
atchat-close-tab = 关闭 { $peer } 标签页
atchat-direct-message = 发给 { $value } 的私信
atchat-cannot-display = 无法显示 { $path }

## CW：带数值的消息（src/app/panels/cw.rs）

cw-click-decoder-neural-one-copies =
    { $hint }

    点击以使用 { $neural } 解码器。神经网络解码器能抄收更远的信号，也能读取时序拟合无法接受的手发 CW；只有时序解码器能抄收带重音的字母——Ä、Ö、Å、Ü、É——因为该模型没有为它们设置输出类别。
cw-idle-s = 空闲 { $idle } 秒

## 解码：带数值的消息（src/app/panels/decodes.rs）

decodes-save-decodes-list-adif-received = 将列表中的 { $n } 条解码保存为 ADIF——即接收报告，而非通联。未指明发送方的解码将被排除。
decodes-save-decodes-list-csv-one = 将列表中的 { $n } 条解码保存为 CSV，每条一行
decodes-session-qso = 本次会话：{ $session } 次通联
decodes-save-whole-logbook-qso-adif = 将整个日志（{ $logged } 次通联）保存为 ADIF
decodes-save-whole-logbook-qso-text = 将整个日志（{ $logged } 次通联）保存为文本
decodes-hz-slots = { $audio_hz } Hz · { $odd } 个时隙
decodes-your-slot-timing-against-stations =
    你的时隙计时与你所收听的电台之间的对比。
    { $after }

    它涵盖整个接收通路，因此缓慢的音频或网络链路与错误的时钟具有同样的影响。低于 0.5 秒较为舒适。
decodes-pile-up = 大批呼叫 { $fox_queue }
decodes-queue = 队列 { $call_queue }
decodes-db-hz-click-remove =
    { $waiting } · { $snr_db } dB · { $audio_hz } Hz{ $unwrap_or_default }
    点击以移除

## JS8：带数值的消息（src/app/panels/js8.rs）

js8-every-speed-being-decoded-marks = 所有速率都在解码；▸ 标记 { $speed }，即你发射所用的速率
js8-every-speed-being-decoded-click = 所有速率都在解码。点击改为以 { $speed } 发射
js8-transmit-decode = 以 { $speed } 发射并解码
js8-beaconing-every-min-click-stop = 每 { $hb_min } 分钟发送一次信标——点击即可停止
js8-last-beacon-went-out-hz = 最近一次信标在 { $hz } Hz 发出——位于 { $value }–{ $value2 } Hz 心跳子频段中的一个空闲时隙，之所以选择它，是为了使其避开正在被解码的信号。
js8-frames-s = { $tx_frames_pending }/{ $tx_frames_total } 帧 · { $left } 秒
js8-decoded = 以 { $speed } 解码
js8-frames = …（{ $frames } 帧）
js8-reply-2 = 回复 { $value }：“{ $value2 }”
js8-address-composer = 将编写框的收件人设为 { $value }

## 宏：带数值的消息（src/app/panels/macros.rs）

macros-f-sends = F{ $i }：发送“{ $text }”
macros-sends = 发送“{ $text }”
macros-most = { $value } 为最多

## NAVTEX：带数值的消息（src/app/panels/navtex.rs）

navtex-repaired-lost = 已修复 { $repaired } · 丢失 { $lost }
navtex-characters-lost = 丢失 { $lost } 个字符

## 分组：带数值的消息（src/app/panels/packet.rs）

packet-bad = { $bad_frames } 个错误
packet-link-layer = 链路层处于 { $link }
packet-via = {" "}经由 { $via }
packet-unacked = { $unacked } 个未确认
packet-retry = 重试 { $retries }

## PI4：带数值的消息（src/app/panels/pi4.rs）

pi4-beacon = { $last_slot_spots } 个信标
pi4-utc-ago = { $t }:{ $t2 } UTC——{ $slot_utc } 前

## RADE：带数值的消息（src/app/panels/rade.rs）

rade-snr-db = 信噪比 { $snr_db } dB
rade-offset-hz = 偏移 { $freq_offset_hz } Hz
rade-heard = 收听到 { $call }
rade-samples-dropped-between-receiver-decoder = ⚠ 接收机与解码器之间丢失了 { $dropped } 个采样——此机器跟不上神经网络解码的速度。

## 文本模式：带数值的消息（src/app/panels/text_modem.rs）

textmodem-char-s = { $chars_per_sec } 字符/秒
textmodem-char-s-hz-wide = { $value }——{ $chars_per_sec } 字符/秒，宽 { $bandwidth_hz } Hz，{ $keyed }

## VDL2：带数值的消息（src/app/panels/vdl2.rs）

vdl2-frames = { $frames } 帧
vdl2-bursts = { $bursts } 次突发
vdl2-sync = { $syncs } 次同步
vdl2-hdlc-bad = HDLC 错误 { $hdlc_bad }
vdl2-bad-fcs = { $fcs_bad } 个 FCS 错误
vdl2-mhz-bursts-frames-noise-floor = { $mhz } MHz，{ $what }——{ $bursts } 次突发，{ $frames } 帧，底噪 { $floor_dbfs } dBFS
vdl2-tune = 调谐 { $mhz }
vdl2-held-2 = 已暂停 +{ $since }
vdl2-use-worldwide-where-every-link = { $what }——全球通用，也是每条链路开始的地方。若只保留一个，请保留这个。
vdl2-assigned-which-channels-carry-anything = 分配给 { $what }。哪些信道承载内容取决于你所在的位置：除非另有所知，请全部保持开启。
vdl2-mhz-db-snr-dbfs-evm = { $mhz } MHz · 信噪比 { $snr_db } dB · { $rssi_dbfs } dBFS · EVM { $evm_deg }° · { $freq_err_hz } Hz · 已纠正 { $rs_corrected } 个 RS 符号
vdl2-characters = { $parity_errors } 个字符
vdl2-parameter-sdroxide-does-not-read = { $unknown } 个 SDRoxide 不读取的参数——见上方，以标识符和字节显示

## WEFAX：带数值的消息（src/app/panels/wefax.rs）

wefax-lines = { $lines } 行
wefax-lines-held = 已保留 { $live_size } 行
wefax-pixels-per-line = 每行 { $width } 个像素
wefax-shift-picture-pixels = 将图片平移 { $px } 个像素
wefax-charts-are-saved-click-copy =
    图表保存在 { $place }

    点击即可复制路径
wefax-finished-charts-are-saved-collect = 完成的图表保存在 { $place }，并收集在此处。
wefax-pixels-right-click-delete =
    { $when_full }
    { $size } × { $size2 } 像素
    { $value }

    右键点击即可删除
wefax-older-store = 存储中另有 { $older } 张更早的图表
wefax-older-load-more = 另有 { $older } 张更早的图表——加载更多
wefax-published-carrier-khz-dial-khz = 公布的载波 { $f } kHz → 表盘 USB { $value } kHz
wefax-delete-chart-2 = 删除此图表 { $dir }

## 部件：带数值的消息（src/app/panels/widgets.rs）

widgets-cq-zone-itu-zone = { $continent } · CQ 区 { $cq_zone } · ITU 区 { $itu_zone }
widgets-new-entity = { $band_label } 上的新实体
widgets-worked-before = 此前已在 { $band_label } 上通联过
widgets-calling-cq-includes-you = 呼叫 CQ { $target }——包括你
widgets-calling-cq-not-aimed-you = 呼叫 CQ { $target }——并非针对你

## WSPR：带数值的消息（src/app/panels/wspr.rs）

wspr-rx-heard-us = { $heard_us } 次接收 · { $heard_us2 } 个收听到了我们
wspr-beacon = { $last_slot_spots } 个信标
wspr-beaconing-duty = 发送信标——{ $wspr_power_dbm }，{ $wspr_tx_percent }% 占空比
wspr-listening-duty = 收听中（{ $wspr_tx_percent }% 占空比）
wspr-beacon-one-two-minute-slot = 每 { $pct } 个两分钟时隙中有一个用于发送信标——占其中的 { $pct2 }%，均匀分布——功率 { $wspr_power_dbm }。周期从哪个时隙开始由你的呼号决定，因此两台运行本程序的电台不会同时开始。
wspr-transmitting-set-general-tab-settings = 以 { $call } 在 { $sent } 发射——请在“设置”的“常规”选项卡中设置。
wspr-utc-ago = { $t }:{ $t2 } UTC——{ $slot_utc } 前

## 公共 SDR：带数值的消息（src/app/publicsdr.rs）

publicsdr-antenna =
    { $address }
    { $device }
    天线：{ $antenna }
    { $e3 }
publicsdr-receivers-fetched-ago = { $entries } 台接收机 · { $per_net } · 获取于 { $age } 前
publicsdr-showing-search-narrow = 显示 { $n } 个中的 { $shown } 个——搜索以缩小范围
publicsdr-match = { $n } 个匹配
publicsdr-map-status = 地图上 { $shown } 个 · 无位置 { $unplaced } 个
publicsdr-opening-another-radio = 正在将 { $entry } 作为另一台电台打开……

## QO-100：带数值的消息（src/app/qo100.rs）

qo100-khz-3 = +{ $park_lo_hz } … +{ $park_hi_hz } kHz
qo100-before-switching-tune-so-beacon = 开启之前：请调谐，使信标的两个波瓣位于条带中的 +{ $park_lo_hz } 与 +{ $park_hi_hz } kHz 之间（阴影通道）。
qo100-receiver-currently-listening-around-mhz = 接收机目前收听的是 { $mhz } MHz 附近——离 10489.750 MHz 信标很远，因此这里没有可绘制的内容。无论如何，解码器都会继续搜索。
qo100-receiver-currently-covers-only-khz = 接收机在此处仅覆盖 ±{ $khz } kHz——条带的其余部分保持空白
qo100-null-db-sym-snr-db = 零点 { $est_null_depth_db } dB 对称 { $est_symmetry } 信噪比 { $est_snr_db } dB
qo100-hz-s-hz-s-de = { $est_drift_hz_s } Hz/s { $est_drift_accel_hz_s2 } Hz/s²（解码前已去旋）
qo100-hz-s-de-rotated-before = { $est_drift_hz_s } Hz/s（解码前已去旋）
qo100-found-empty = 找到 { $est_updates } 次，空 { $est_misses } 次
qo100-last-s-ago = {"   "}上次 { $auto_last_hz }（{ $auto_last_unix } 秒前）
qo100-over-correction = 累计 { $auto_total_hz }，共 { $auto_applies } 次校正{ $last }
qo100-sync-32-err = 同步 { $sync_bit_errors } / 32 个错误 ×{ $sync_matches }
qo100-frame-buffer = 帧缓冲 { $frame_fill }%
qo100-blocks-tried-decoded = 已尝试 { $blocks_tried } 个数据块，已解码 { $blocks_locked } 个
qo100-last-applied-s-ago-hz = 上次应用于 { $ago } 秒前：{ $old } → { $new } Hz（{ $old2 } Hz）
qo100-locked-block-tried-locked = 已锁定——已尝试 { $blocks_tried } 个数据块，锁定 { $blocks_locked } 个
qo100-searching-block-tried-none-locked = 搜索中——已尝试 { $blocks_tried } 个数据块，尚未锁定任何一个

## RDS：带数值的消息（src/app/rds.rs）

rds-auto = 自动（{ $standard }）
rds-type = 类型 { $class }
rds-groups = · { $groups } 个组
rds-blocks-error = · { $ber }% 的数据块出错
rds-clean-corrected-lost = { $blocks_ok } 个正确 · { $blocks_corrected } 个已纠正 · { $blocks_bad } 个丢失

## 卫星：带数值的消息（src/app/sat.rs）

sat-utc-local = { $unix } UTC（本地 { $offset }）
sat-aos = AOS { $rise_unix } { $rise_unix2 }
sat-los = LOS { $set_unix } { $set_unix2 }
sat-above-horizon = { $rise_unix } 在地平线以上
sat-min = { $value } 分钟后
sat-h-min = { $value } 小时 { $value2 } 分钟后
sat-searching = 搜索中…… { $next } / { $entries }
sat-locked-2 = ● 已锁定 — { $t }
sat-pass-until-max = 过境至 { $set_unix } · 最大仰角 { $max_el }°
sat-next-pass-rises-max = 下一次过境 { $rise_unix } { $rise_unix2 }（{ $now }）· 升起于方位 { $rise_az }° { $rise_az2 } · 最大仰角 { $max_el }°
sat-rigctld-client-connected-satellite-software = ⚠ 已连接 { $clients } 个 rigctld 客户端——卫星软件若在那里操控表盘，会对多普勒进行两次校正
sat-rotator-antenna-az-el = 旋转器：天线位于方位 { $az }°、仰角 { $el }°
sat-rotator-2 = 旋转器：{ $error }
sat-up-now-2 = ● 当前在空，仰角 { $el }°
sat-inv = { $mode } · 反向
sat-norad-2 = NORAD { $norad_id }

## 扫描器：带数值的消息（src/app/scanner.rs）

scanner-holding-mhz = 停留在 { $here } MHz
scanner-scanning-mhz = 扫描中 · { $here } MHz

## 频谱：带数值的消息（src/app/spectrum.rs）

spectrum-renderer-will-not-hold-texture = 此渲染器无法保存宽度超过 { $max_texture_dim } 像素的纹理。
spectrum-mem = 记忆：{ $f } / { $value }
spectrum-mem-2 = 记忆：{ $value }

## 语音：带数值的消息（src/app/speech.rs）

speech-voice-2 = 语音：{ $voice }
speech-speech-unavailable = 语音不可用：{ $error }

## 信号列表：带数值的消息（src/app/spots.rs）

spots-transmissions = { $stations } 次发射

## Winlink：带数值的消息（src/app/winlink.rs）

winlink-last-session-failed = 上次会话失败——{ $err }
winlink-last-session-received-sent = 上次会话：接收 { $last_received } 条，发送 { $last_sent } 条
winlink-empty = { $to_lowercase } 为空
winlink-showing = 显示 { $total } 条中的 { $entries } 条
winlink-fetching = 正在获取 { $mid }……
winlink-from-2 = 来自 { $msg } { $date }
winlink-attachment-bytes = 附件：{ $att }（{ $data } 字节）

## APRS 地图：带数值的消息（src/aprs_map.rs）

aprsmap-object =
    {""}
    来自 { $reported_by } 的对象
aprsmap-heard-ago =
    {""}
    { $age } 前收听到

## 下载：带数值的消息（src/download.rs）

download-reading = 正在读取 { $path }：{ $error }

## 帮助：带数值的消息（src/help.rs）

help-missing-image = {"["}缺少图像：{ $path }]

## HFDL 地图：带数值的消息（src/hfdl_map.rs）

hfdlmap-mhz =
    {""}
    { $e3 } MHz
hfdlmap-fix =
    {""}
    { $fixes } 次修正
hfdlmap-heard-ago =
    {""}
    { $last_at } 前收听到

## 多电台：带数值的消息（src/multi.rs）

multi-radio = 电台 { $n }
multi-could-not-add-radio = 无法添加电台：{ $error }
multi-was-closed-station = { $value } 已在电台站点上关闭。
multi-could-not-open = 无法打开 { $peer }：{ $error }
multi-has-tab-own-now = { $value } 现在拥有其自己的选项卡。

## 远程：带数值的消息（src/remote.rs）

remote-protocol-error = 协议错误：{ $error }

## 太阳系网络：带数值的消息（src/solar3d/net.rs）

solarnet-bad-message = 错误的消息：{ $error }
solarnet-solar-protocol-mismatch-server-client = 太阳协议不匹配：服务器 { $proto }，客户端 { $value }

## S 表：带数值的消息（src/widgets/smeter.rs）

smeter-forward-power-w-measured-radio =
    {""}

    正向功率——{ $w } W，由电台自身的传感器测得。
smeter-po-radio-s-own-power =
    {""}

    PO——电台自身的输出功率表，位于其满量程的 { $clamp }%。这是电台报告的指针位置，而不是经过校准的瓦数；应当观察的是，在驱动保持不变的情况下它是否下降。
smeter-swr-1-match-antenna-presenting =
    {""}

    驻波比——{ $swr }:1，即天线呈现的匹配程度，由电台测量。低于 2:1 不值得注意；超过 3:1 时刻度将变为红色。
smeter-received-signal-strength-s9-db = 接收信号强度：S9 + { $over } dB（通带内 { $s_dbm } dBm）。
smeter-received-signal-strength-s-dbm = 接收信号强度：S{ $value }（通带内 { $s_dbm } dBm）。
smeter-radio-reports-c =
    {""}

    电台报告 { $t } °C。
smeter-ps-puresignal-correcting-db-compression =
    {""}

    PS——PureSignal 正在校正 { $correction_db } dB 的压缩{ $held }（反馈与发射匹配，得分 { $score }）。
smeter-ps-puresignal-switched-but-has =
    {""}

    PS −− ——PureSignal 已开启，但尚未在反馈通路中找到发射信号（最佳匹配 { $score }），因此发射机输出的是未经校正的信号。请检查功放输出的一份采样是否确实到达了一个发射时不会被收发开关切断的接收输入，以及其衰减器是否将其吞没。环路仅监听此接收机——接到第二台接收机（RX2、ADC2）的耦合器永远不会被读取为反馈。

## 频谱视图：带数值的消息（src/widgets/spectrum_view.rs）

specview-rit-hz = RIT { $hz } Hz
specview-xit-hz = XIT { $hz } Hz

## 解码：嵌套消息

decodes-you-transmit-s-everyone-else = 你的发射比其他所有人 { $off } 秒 { $after }——这是呼叫常常得不到应答的常见原因。请同步你计算机的时钟。

## 公共 SDR：嵌套消息

publicsdr-noise-floor-score = 底噪得分 { $snr }
publicsdr-up-khz-i-q = I/Q 最高 { $khz } kHz

## 发射音频：更多消息

txaudio-gr-gate-de-ess-comp = 增益衰减 噪声门 { $gate_gr_db } 去齿音 { $deess_gr_db } 压缩 { $comp_gr_db } 限幅 { $limiter_gr_db }

## 功放：更多消息

amp-h-step = ~{ $l_step } µH · 步进 { $l_step2 }
amp-pf-step = ~{ $c_step } pF · 步进 { $c_step2 }

## SSTV：更多消息

sstv-bit-octets-chunks-first-pass = { $from } · { $filename } · { $width }×{ $height } { $bits_per_pixel } 位 · { $media_type } / { $content_encoding } · { $encoded_size } 字节，分为 { $chunk_count } 个数据块（首轮 { $chunks_first_pass }）· 会话 { $session }
sstv-baud-cpfsk-hz-khz-occupied = { $value }——{ $symbol_rate } 波特 CPFSK，±{ $deviation_hz } Hz，占用带宽 { $bandwidth_hz } kHz

## 时钟，以及若干带数值的短标签。

time-utc-and-local = { $utc } UTC · 本地 { $local }
sat-in-seconds = { $s } 秒后
winlink-to-list = 收件人 { $to }
winlink-cc-list = 抄送 { $cc }
spots-n-of-total = { $n } / { $total }
wefax-n-of-total = { $n } / { $total }
adsb-range-bearing = { $km } km，方位 { $bearing }°
ais-range-bearing = { $km } km，方位 { $bearing }°

## sdroxide-types 中的更多取值，由选择器显示其名称（src/i18n/labels.rs）。
rtl-sdr-hf-mode-off = 关闭（仅调谐器，24 MHz 以上）
rtl-sdr-hf-mode-auto = 自动
rtl-sdr-hf-mode-direct-q = 直接采样（Q 支路）
rtl-sdr-agc-manual = 手动（无 AGC）
rtl-sdr-agc-tuner = 调谐器 AGC
rtl-sdr-agc-rtl = RTL 数字 AGC
rtl-sdr-agc-both = 调谐器 + RTL AGC
mode-control-cat = CAT
mode-control-radio = 由电台控制
line-state-none = 无
line-state-high = 高
line-state-low = 低
elad-tx-input-usb-audio = USB 音频
elad-tx-input-mic = 话筒
elad-tx-input-auto = 自动
elad-tx-input-leave = 保持电台上的设置
tr-usdx-audio-one-cable = 单根线缆（音频在 CAT 数据流中）
tr-usdx-audio-sound-card = USB 声卡（3.5 mm 插孔）
spy-server-format-uint8 = 8 位（最低带宽）
spy-server-format-int16 = 16 位
spy-server-format-float32 = 32 位浮点
sound-format-iq = IQ（立体声）
sound-format-demod-audio = 解调音频
sdr-play-duo-tuner-tuner1 = 调谐器 1（50 Ohm / Hi-Z）
sdr-play-duo-tuner-tuner2 = 调谐器 2（50 Ohm）
sdr-play-duo-role-diversity = 第二根天线（分集 / 抑制干扰）
sdr-play-duo-role-second-radio = 第二台电台，使用其自己的频率
sdr-play-agc-off = 关闭（手动中频增益）
sdr-play-agc-hz5 = 5 Hz
sdr-play-agc-hz50 = 50 Hz
sdr-play-agc-hz100 = 100 Hz
rotator-protocol-rotctld = Hamlib rotctld（TCP）
rotator-protocol-pst-rotator = PSTRotator（UDP）
pluto-ptt-off = 关闭
pluto-ptt-gpo01 = GPO0 = 接收，GPO1 = 发射
pluto-ptt-gpo23 = GPO2 = 接收，GPO3 = 发射
pluto-duplex-fdd = FDD（同时双向）
pluto-duplex-tdd = TDD（一次一个方向）
pluto-agc-manual = 手动
pluto-agc-slow-attack = 慢速起振
pluto-agc-fast-attack = 快速起振
pluto-agc-hybrid = 混合
parity-none = 无
parity-even = 偶
parity-odd = 奇
panadapter-tap-antenna = 天线（与电台共用）
panadapter-tap-if-output = 电台的中频输出
panadapter-audio-attached = 所连接的接收机
panadapter-audio-transceiver = 收发信机
kenwood-send-ts2000 = TS-2000 风格（TX;）
kenwood-send-ts590 = TS-590 风格（TX1;）
digi-mode-usb = USB
digi-mode-data = DIGI
digi-mode-radio = 由电台控制
cw-keying-cat = 电台键控器（CAT）
cw-keying-audio = 声卡（MCW）
audio-route-both = 两个声道
audio-route-left = 仅左声道
audio-route-right = 仅右声道
amp-model-hr500 = Hardrock-500
amp-model-hr50 = Hardrock-50（未测试）
alert-sound-ding = 叮声
alert-sound-two-tone = 双音
alert-sound-triplet = 三连音
alert-sound-warble = 颤音
alert-sound-digital = 数字
alert-reply-tone = 音调
alert-reply-voice = 语音
alert-reply-both = 音调 + 语音
icom-model-ic7300 = IC-7300
icom-model-ic7300-mk2 = IC-7300MK2
icom-model-ic705 = IC-705
icom-model-ic7760 = IC-7760
icom-model-ic905 = IC-905
icom-model-ic9700 = IC-9700
icom-model-ic7610 = IC-7610
icom-model-ic7100 = IC-7100
icom-model-ic7410 = IC-7410
icom-model-ic7600 = IC-7600
icom-model-ic7700 = IC-7700
icom-model-ic7800 = IC-7800
icom-model-ic9100 = IC-9100
icom-model-ic7200 = IC-7200
icom-model-ic7000 = IC-7000
icom-model-other = 其他（手动设置地址）
backend-auto = 自动检测（SoapySDR / CAT）
backend-soapy = SoapySDR
backend-cat = CAT / 音频
backend-hpsdr = HPSDR（网络）
backend-tci = TCI（网络）
backend-icom-net = Icom LAN（网络）
backend-smart-sdr = SmartSDR / FlexRadio（网络）
backend-pluto = PlutoSDR（网络）
backend-rtl-sdr = RTL-SDR（USB）
backend-rtl-tcp = 通过 rtl_tcp 的 RTL-SDR（网络）
backend-spy-server = SpyServer（网络）
backend-spy-server-vfo = SpyServer VFO+FFT，低带宽（网络）
backend-kiwi-sdr = KiwiSDR / Web-888（网络）
backend-rx888 = RX-888（USB）
backend-airspy-hf = Airspy HF+（USB）
backend-airspy = Airspy R2 / Mini（USB）
backend-hack-rf = HackRF One / Pro（USB）
backend-sdr-play = SDRplay RSP（USB）
backend-elad = ELAD FDM-DUO / FDM-S（USB）
backend-lime = LimeSDR + LimeRFE（LimeSuite）
backend-hydra-sdr = HydraSDR RFOne（USB）
backend-fobos = RigExpert Fobos SDR（USB）
backend-usb-audio = USB 音频电台（声卡）
backend-none = 未配置
cat-family-xiegu = Xiegu
cat-family-icom = Icom
cat-family-yaesu = Yaesu
cat-family-kenwood = Kenwood
cat-family-elecraft = Elecraft
cat-family-elad = ELAD
cat-family-qrp-labs = QRP Labs
cat-family-rs-hfiq = RS-HFIQ
cat-family-tr-usdx = (tr)uSDX
cat-family-rigctld = Hamlib rigctld（网络）
cat-family-flrig = flrig（网络）

## sdroxide-types 中的取值，显示在标签、菜单和列表上（src/i18n/labels.rs）。
agc-mode-off = 关
agc-mode-slow = 慢
agc-mode-med = 中
agc-mode-fast = 快
airspy-gain-linearity = 线性（最佳强信号处理）
airspy-gain-sensitivity = 灵敏度（最佳弱信号）
ais-kind-class-a = A 类
ais-kind-class-b = B 类
ais-kind-base-station = 基站
ais-kind-sar-aircraft = 搜救飞机
ais-kind-aid-to-navigation = 助航设备
ais-kind-sart = SART / MOB / EPIRB
ais-kind-craft = 关联船只
alert-event-called = 有电台在呼叫我
alert-event-cq = 我能应答的 CQ
alert-event-new-dxcc = 新的 DXCC 实体
alert-event-new-dxcc-band = 此频段上的新实体
alert-event-new-grid = 新的网格方格
audio-routing-both = 左 + 右
audio-routing-left = 仅左
audio-routing-right = 仅右
button-mode-momentary = 按住
button-mode-toggle = 切换
contest-mode-none = 无
contest-mode-eu-vhf = EU VHF 比赛
converter-tx-off = 转换时关闭
converter-tx-transverter = 通过同一变频器
converter-tx-own = 独立偏移
cw-engine-neural = 神经网络
cw-engine-timing = 时序
decode-sort-none = 无
decode-sort-signal = 信噪比
decode-sort-distance = 距离
decode-sort-country = 国家
diversity-mode-cancel = 消除——使噪声源置零
diversity-mode-combine = 合并——分集接收
dxped-mode-normal = 普通
dxped-mode-hound = Hound
dxped-mode-fox = Fox
fail-safe-refuse-tx = 拒绝发射
fail-safe-warn-only = 仍然发射，但发出警告
ftp-security-plain = FTP（无加密）
ftp-security-explicit-tls = FTPS 显式（AUTH TLS）
ftp-security-implicit-tls = FTPS 隐式
hpsdr-filter-board-none = 无——输出保持关闭
hpsdr-filter-board-n2adr = N2ADR 滤波器板
hpsdr-filter-board-alex = Alex / Hermes 频段代码（Zeus、HiQSDR、Quisk）
hpsdr-filter-board-custom = 自定义——我在下方自己的表格
hpsdr-io-rx-input-radio = 电台自身的输入
hpsdr-io-rx-input-io-board = IO 板 J9
hpsdr-io-rx-input-io-board-pure-signal = IO 板 J9，发射时使用 PureSignal
hydra-sdr-gain-linearity = 线性（最佳强信号处理）
hydra-sdr-gain-sensitivity = 灵敏度（最佳弱信号）
icom-rx-source-af = AF——电台解调
icom-rx-source-if12k = 12 kHz 中频——sdroxide 解调
icom-scope-span-radio = 保持电台上的设置
icom-scope-span-khz2-5 = 5 kHz（±2.5k）
icom-scope-span-khz5 = 10 kHz（±5k）
icom-scope-span-khz10 = 20 kHz（±10k）
icom-scope-span-khz25 = 50 kHz（±25k）
icom-scope-span-khz50 = 100 kHz（±50k）
icom-scope-span-khz100 = 200 kHz（±100k）
icom-scope-span-khz250 = 500 kHz（±250k）
icom-scope-span-khz500 = 1 MHz（±500k）
ism-family-weather = 气象
ism-family-meter = 仪表
ism-family-home-auto = 家居
ism-family-lo-ra = LORA
ism-family-unidentified = 未知
ism-protocol-la-crosse-it = LaCrosse IT+
ism-protocol-fine-offset = Fine Offset
ism-protocol-bresser = Bresser
ism-protocol-z-wave = Z-Wave
ism-protocol-homematic = Homematic
ism-protocol-rtl433 = rtl_433
ism-protocol-unidentified = 未识别
js8-speed-normal = 普通
js8-speed-fast = 快速
js8-speed-turbo = Turbo
js8-speed-slow = 慢速
lime-aux-role-off = 未使用
lime-aux-role-diversity = 第二根天线（分集 / 抑制干扰）
lime-aux-role-pure-signal = 发射反馈（PureSignal 预失真）
lookup-provider-none = 关闭
lookup-provider-qrz = QRZ.com
lookup-provider-ham-qth = HamQTH
mail-folder-inbox = 收件箱
mail-folder-outbox = 发件箱
mail-folder-sent = 已发送
mail-folder-archive = 存档
memory-sort-name = 名称
memory-sort-freq = 频率
memory-sort-band = 频段
memory-sort-stored = 存储顺序
mouse-button-middle = 中键
mouse-button-extra1 = 后退（额外 1）
mouse-button-extra2 = 前进（额外 2）
nr-level-off = 关
nr-level-low = 低
nr-level-medium = 中
nr-level-high = 高
nr-level-rnn-low = RNN 低
nr-level-rnn-med = RNN 中
nr-level-rnn-high = RNN 高
nr-level-spec-low = SPEC 低
nr-level-spec-med = SPEC 中
nr-level-spec-high = SPEC 高
nr-level-nr2-low = NR2 低
nr-level-nr2-med = NR2 中
nr-level-nr2-high = NR2 高
nr-level-df-low = DFNR 低
nr-level-df-med = DFNR 中
nr-level-df-high = DFNR 高
nr-strength-low = 低
nr-strength-med = 中
nr-strength-high = 高
orbit-rings-none = 无
orbit-rings-curated = 精选
orbit-rings-all = 全部
qso-step-idle = 空闲
qso-step-wait-cq = 等待 CQ
qso-step-calling-cq = 呼叫 CQ
qso-step-tx-grid = 发送网格
qso-step-tx-report = 发送报告
qso-step-tx-r-report = 发送 R+报告
qso-step-tx-rr73 = 发送 RR73
qso-step-tx73 = 发送 73
qso-step-confirming = 确认中
rds-standard-auto = 自动
rds-standard-rds = RDS
rds-standard-rbds = RBDS
region-r1 = 第 1 区——欧洲、非洲、中东、北亚
region-r2 = 第 2 区——美洲
region-r3 = 第 3 区——南亚和东亚、大洋洲、太平洋
relative-mode-absolute = 绝对（推子）
relative-mode-sign-magnitude = 相对：符号/幅值
relative-mode-twos-complement = 相对：二进制补码
relative-mode-binary-offset = 相对：以 64 为中心
relay-link-off = 无
relay-link-serial = USB 继电器板（串口）
relay-link-serial-lines = 串口 RTS/DTR 线路
relay-link-hid = USB 继电器板（HID）
relay-link-cm108 = CM108/CM119 声卡 GPIO
relay-link-gpio = Linux GPIO 线路
relay-link-command = 外部命令
relay-role-unused = 未使用
relay-role-sdr-antenna = 接地 SDR 天线
relay-role-amplifier = 按下功放 / 收发继电器
relay-role-aux = 辅助
relay-role-band-decoder = 频段解码器（滤波器 / 变频器）
rfe-channel-wb1000 = 宽带 1 – 1000 MHz
rfe-channel-wb4000 = 宽带 1 – 4 GHz
rfe-channel-ham0030 = HF，最高 30 MHz
rfe-channel-ham0070 = 6 m / 4 m（50 – 70 MHz）
rfe-channel-ham0145 = 2 m（140 – 150 MHz）
rfe-channel-ham0220 = 1.25 m（220 – 225 MHz）
rfe-channel-ham0435 = 70 cm（400 – 450 MHz）
rfe-channel-ham0920 = 33 cm（902 – 928 MHz）
rfe-channel-ham1280 = 23 cm（1220 – 1420 MHz）
rfe-channel-ham2400 = 13 cm（2.3 – 2.5 GHz）
rfe-channel-ham3500 = 9 cm（3.3 – 3.7 GHz）
rfe-channel-cell-band01 = 蜂窝频段 1
rfe-channel-cell-band02 = 蜂窝频段 2
rfe-channel-cell-band03 = 蜂窝频段 3
rfe-channel-cell-band07 = 蜂窝频段 7
rfe-channel-cell-band38 = 蜂窝频段 38
rfe-channel-not-selected = 未选择
rfe-link-off = 未连接
rfe-link-serial = 其自己的 USB 线缆（串口）
rfe-link-board = 通过 LimeSDR（GPIO / I²C）
rfe-mode-control-auto = 自动（跟随接线）
rfe-mode-control-rx = 始终接收
rfe-mode-control-tx = 始终发射
rfe-mode-control-tx-rx = 始终两者（仅限分离式连接器）
rifp-encoding-auto = 自动
rifp-encoding-png = PNG
rifp-encoding-jpeg = JPEG
rifp-encoding-raw = 原始
rifp-encoding-rle8 = RLE8
rifp-encoding-zlib = Zlib
rifp-encoding-group4 = G4
rifp-encoding-group3 = G3
scan-kind-memories = 记忆
scan-kind-range = 范围
scan-resume-carrier = 载波
scan-resume-timed = 定时
scan-resume-manual = 手动
sense-line-off = 未接线
sense-line-cts = CTS
sense-line-dsr = DSR
sense-line-dcd = DCD（载波检测）
shift-simplex = 单工
shift-minus = −
shift-plus = +
speed-slow = 慢
speed-medium = 中
speed-fast = 快
speed-faster = 更快
speed-fastest = 最快
tls-verify-system = 系统信任库
tls-verify-pinned = 固定指纹
tls-verify-off = 不验证
tone-mode-off = 关
tone-mode-ctcss = CTCSS
tone-mode-dcs = DCS
tx-voice-stage-gate = 噪声门
tx-voice-stage-eq = 均衡器
tx-voice-stage-de-ess = 去齿音器
tx-voice-stage-comp = 压缩器
upload-naming-sequence = image1 … imageN（循环）
upload-naming-original = 原始文件名
wheel-action-zoom = 缩放
wheel-action-tune = 调谐
wheel-action-none = 无

# Undocked windows (from sdroxide-brown)
frame-detached-title = { $flavor }：{ $name }
frame-dock-band-menu-hint = 停靠：把波段和模式选择器放回主窗口，效果与关闭此窗口相同，无需寻找关闭按钮。
frame-dock-panel-hint = 停靠：把此面板放回瀑布图下方
frame-dock-spectrum-hint = 停靠：把此频谱放回主窗口，效果与关闭此窗口相同，无需寻找关闭按钮。
frame-every-module-own-window = 每个模块都在各自的窗口中
frame-module-aux-panadapter = 辅助全景显示
frame-module-band-mode = 波段与模式
frame-module-controls = 控制
frame-module-panadapter = 全景显示
frame-module-panel = 面板
multi-dock-all-windows = 全部停靠
multi-module-panel = 操作面板
multi-undock-all-windows = 全部取消停靠
settings-ui-dock-all-hint = 把所有已取消停靠的窗口放回此窗口。
settings-ui-docked = 停靠在此窗口中
settings-ui-module-aux = 辅助频谱（第二频谱）
settings-ui-module-band-menu = 波段和模式选择器
settings-ui-module-controls = 控制
settings-ui-module-panadapter = 全景频谱
settings-ui-undock-all-hint = 让每个模块都使用自己的窗口：主窗口只留下频谱和解码器。没有内容可显示的模块，例如语音模式下的操作面板，不会打开窗口。
settings-ui-undock-hint =
    取消停靠后，此模块将移到独立窗口，主窗口把空出的空间让给其余部分：可以把瀑布图放到第二块显示器，把控制面板放到第三块。关闭其窗口即可重新停靠。

    在Wayland下大小可以恢复，但窗口位置由合成器决定：请让它浮动，并用匹配app-id "{ $app_id }" 的窗口规则把它固定到某块显示器。
settings-ui-undocked = 已取消停靠：独立窗口
settings-ui-window-label = { $module }窗口
frame-window-chip-hint = 窗口：把此窗口移到独立的系统窗口，以便放到另一个显示器上
frame-dock-panel-pane-hint = 停靠：把此面板放回瀑布图下方的窗格
frame-panel-window-title = { $mode }：面板
frame-roll-up-hint = 卷起：离开标题栏，把窗口还给瀑布图
frame-unroll-hint = 展开：重新显示面板
frame-window-panel-hint = 窗口：给此面板一个独立窗口，以便放到另一个显示器上
topbar-dock-all = 全部停靠
topbar-dock-all-hint = 将所有模块窗口收回此窗口
topbar-module-windows-title = 模块窗口
topbar-undock-all = 全部取消停靠
topbar-undock-all-hint = 让每个模块都有独立窗口：此窗口只剩频谱和解码器
topbar-windows-chip-hint =
    ▣  左键：将所有模块窗口收回
    右键：模块窗口列表
topbar-wname-aux = SP2（AUX）：第二频谱
topbar-wname-band-menu = 波段和模式
topbar-wname-controls = 接收控制
topbar-wname-panadapter = SP1：频谱
topbar-wname-panel = 操作面板
topbar-wstate-float = 将 { $name } 从面板中提出，浮动在此窗口之上
topbar-wstate-floating-panel-only = 浮动是操作面板自身的状态：频谱或控制台在此窗口中只有一个位置
topbar-wstate-here-now = { $name } 现在就在这里
topbar-wstate-no-panel = { $mode } 中没有操作面板：在有操作面板的模式下才适用
topbar-wstate-own-window = 为 { $name } 提供独立窗口，可放在任意显示器上
topbar-wstate-put-back = 将 { $name } 放回此窗口
frame-workspace-missing = 未保存工作区 { $n }。请在“设置 → 界面 → 工作区”中保存一个，此按键就会调出它。
settings-ui-workspaces = 工作区
settings-ui-workspaces-note = 一个有名称的窗口布局：哪些模块在独立窗口中，以及这些窗口的位置。可以保存当前布局，或恢复已保存的布局。最多十个。
settings-ui-workspaces-none = 尚未保存任何工作区。
settings-ui-workspace-restore-home = 恢复此布局：所有模块都在主窗口中
settings-ui-workspace-restore-out = 恢复此布局：{ $n } 个模块位于独立窗口
settings-ui-workspace-forget = 删除此工作区
settings-ui-workspace-name = 为此布局命名
settings-ui-workspace-full = SDRuno最多保留十个工作区，请先删除一个
settings-ui-workspace-save = 保存当前布局
settings-ui-workspace-save-hint = 以此名称保存当前布局
settings-ui-workspace-name-first = 请先输入名称
topbar-band-window2-hint = 窗口：在独立窗口中打开选择器，宽度随意，可放在任意显示器上
tnc-link = 链路
tnc-connected = 已连接
tnc-calling = 呼叫中
tnc-idle = 守听中
tnc-tx = TX
tnc-busy = 忙
tnc-busy-hint = 信道上有其他电台。现在呼叫会压在对方信号上。
tnc-setup = ⚙ 设置
tnc-peer = 对方
tnc-width = 带宽
tnc-speed = 速率
tnc-snr = 信噪比
tnc-bytes = 字节
tnc-queued = 待发
tnc-host = 主机
tnc-ports = 端口
tnc-ports-off = 关闭
tnc-nobody = 无
tnc-holder-panel = 本面板
tnc-holder-winlink = MAIL 窗口
tnc-holder-tcp = 位于 { $addr } 的程序
tnc-tune = 调谐
tnc-abort = 中止
tnc-abort-hint = 立即断开链路,无论由谁控制。对方不会收到通知,只会超时。
tnc-terminal = 终端
tnc-callsign = 呼号
tnc-connect = 连接
tnc-disconnect = 断开
tnc-disconnect-hint = 正常挂断:先发完排队的数据,再等对方确认。
tnc-held-by = 调制解调器正被{ $holder }使用。
tnc-no-callsign = 请先设置呼号(⚙ 设置)
tnc-clear = 清除
tnc-clear-hint = 清空记录。链路不受影响。
tnc-nothing-yet = 暂无内容。在上方输入呼号并点击连接,或在 ⚙ 设置中打开守听以接听呼叫。
tnc-type-here = 在此输入,回车发送
tnc-not-connected = 未连接
tnc-send = 发送
setup-tnc-bandwidth = 带宽
setup-tnc-listen = 接听呼叫
setup-tnc-listen-check = 在面板中守听
setup-tnc-listen-hint = 在面板终端中接听来电。TCP 主机端口开启时无效,因为连接在那里的程序要接听呼叫。
setup-tnc-host = TCP 主机
playback-radio-n = { " " }· 电台 { $n }
playback-no-stamp = 名称中没有时间戳，将以当前模式播放
playback-title = 回放：已录音频
playback-find-hint = 模式、MHz、日期、名称…
playback-refresh-hint = 重新读取录音文件夹
playback-asking = 正在向电台询问其录音…
playback-folder = 文件夹：{ $dir }
playback-none = 还没有录音。在有信号时按 录制 → MP3；文件会出现在这里，以其时间、频率和模式命名。
playback-on = 播放中
playback-play = 播放
playback-iq-hint = 这是 I/Q 捕获，不是音频。回放使用录音机写入的音频；I/Q 捕获请用 --file 播放。
playback-not-while-tx = 发射时不可用
playback-play-hint = 从头将此录音播放到解码器中
playback-busy-playing = 正在回放，请先停止再删除
playback-busy-recording = 仍在录制，请先停止 录制 再删除
playback-delete-ask = 删除？
playback-delete-hint-armed = 再按一次将永久删除此文件，无法撤销。
playback-delete-hint = 从文件夹中删除此录音（会再确认一次）
playback-strip-label = 回放
playback-not-live = 非实时，电台已让出
playback-carry-on = 继续播放
playback-pause = 暂停
playback-pause-hint = 暂停，解码器在您继续之前听到的是静音
playback-again = 重来
playback-again-hint = 回到开头，再试一次图像或解码
playback-loop = 循环
playback-loop-hint = 文件结束时从头重新开始
playback-stop = 停止
playback-stop-hint = 结束回放并让电台恢复原状
playback-files = 文件
playback-end-of-file = 文件结束
playback-paused = 已暂停
playback-recorded-on-mode = 录制于 { $mhz } MHz，{ $mode } 模式，拨盘和模式已根据文件名设置。
playback-recorded-on = 录制于 { $mhz } MHz，拨盘和模式已根据文件名设置。
playback-no-dial-mode = 文件名中没有记录拨盘或模式，因此以当前模式播放，请从模式菜单选择解码器。
playback-format-stereo = { $rate } Hz，立体声，播放左（接收）声道。
playback-format-mono = { $rate } Hz，单声道，播放该唯一声道。
playback-stopped = 已停止：{ $fault }
playback-intro = 将录音回放到解码器中，包括 SSTV、CW、RTTY、PSK、FT8 以及其他所有从音频解码的模式。播放期间电台让出，拨盘和模式会调到录音时的设置，停止 会让电台恢复。播放时更改模式可尝试另一个解码器。
topbar-stereo-yields-hint = 降噪或自动陷波运行时，WFM立体声关闭，因为它们会模糊两个声道。关闭它们即可收听立体声
topbar-stereo-playback-hint = 立体声：录音按录制时的样子在双耳中回放。点击强制为单声道
topbar-stereo-mono-recording-hint = 此录音为单声道：没有第二个声道可播放
topbar-caption-play-back = 回放
topbar-recordings-chip = ▶ 录音…
topbar-recordings-hint = 将录音回放到解码器中：错过的SSTV图片、微弱的RTTY或CW信号、一个FT8时隙，配合瀑布图和解码日志，如同正在空中接收。回放期间电台暂停工作。
playback-find = 查找
playback-refresh = 刷新
