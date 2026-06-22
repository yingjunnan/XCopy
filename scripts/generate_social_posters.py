from pathlib import Path
import math

from PIL import Image, ImageDraw, ImageFilter, ImageFont


ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "output" / "social"
ICON = ROOT / "src-tauri" / "icons" / "128x128.png"

W, H = 1080, 1920

BLUE = (0, 103, 192)
BLUE_DARK = (0, 76, 148)
GREEN = (16, 124, 16)
PURPLE = (135, 100, 184)
RED = (196, 43, 28)
INK = (15, 23, 42)
INK_2 = (71, 85, 105)
INK_3 = (100, 116, 139)
LINE = (226, 232, 240)
BG = (248, 250, 252)
WHITE = (255, 255, 255)


def font(size, bold=False, light=False):
    if bold:
        candidates = [
            "C:/Windows/Fonts/msyhbd.ttc",
            "C:/Windows/Fonts/Dengb.ttf",
            "C:/Windows/Fonts/simhei.ttf",
        ]
    elif light:
        candidates = [
            "C:/Windows/Fonts/msyhl.ttc",
            "C:/Windows/Fonts/Dengl.ttf",
            "C:/Windows/Fonts/msyh.ttc",
        ]
    else:
        candidates = [
            "C:/Windows/Fonts/msyh.ttc",
            "C:/Windows/Fonts/Deng.ttf",
            "C:/Windows/Fonts/simhei.ttf",
        ]
    for path in candidates:
        if Path(path).exists():
            return ImageFont.truetype(path, size)
    return ImageFont.load_default()


def rgba(color, alpha, bg=WHITE):
    """Return an opaque color visually equivalent to alpha blending on white."""
    a = max(0, min(255, alpha)) / 255
    return tuple(int(bg[i] * (1 - a) + color[i] * a) for i in range(3))


def gradient(size, top, bottom):
    img = Image.new("RGBA", size, top)
    px = img.load()
    for y in range(size[1]):
        t = y / max(1, size[1] - 1)
        row = tuple(int(top[i] * (1 - t) + bottom[i] * t) for i in range(4))
        for x in range(size[0]):
            px[x, y] = row
    return img


def rounded_mask(size, radius):
    mask = Image.new("L", size, 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, size[0], size[1]), radius=radius, fill=255)
    return mask


def shadow(base, box, radius=28, blur=34, alpha=56, offset=(0, 18)):
    x1, y1, x2, y2 = box
    layer = Image.new("RGBA", base.size, (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)
    moved = (x1 + offset[0], y1 + offset[1], x2 + offset[0], y2 + offset[1])
    d.rounded_rectangle(moved, radius=radius, fill=(15, 23, 42, alpha))
    base.alpha_composite(layer.filter(ImageFilter.GaussianBlur(blur)))


def rounded_rect(draw, box, radius, fill, outline=None, width=1):
    draw.rounded_rectangle(box, radius=radius, fill=fill, outline=outline, width=width)


def text_size(draw, text, fnt):
    box = draw.textbbox((0, 0), text, font=fnt)
    return box[2] - box[0], box[3] - box[1]


def center_text(draw, x, y, text, fnt, fill, anchor="mm"):
    draw.text((x, y), text, font=fnt, fill=fill, anchor=anchor)


def wrap_text(draw, text, fnt, max_width):
    lines = []
    current = ""
    for ch in text:
        test = current + ch
        if text_size(draw, test, fnt)[0] <= max_width:
            current = test
        else:
            if current:
                lines.append(current)
            current = ch
    if current:
        lines.append(current)
    return lines


def draw_wrapped(draw, xy, text, fnt, fill, max_width, line_gap=10):
    x, y = xy
    for line in wrap_text(draw, text, fnt, max_width):
        draw.text((x, y), line, font=fnt, fill=fill)
        y += text_size(draw, line, fnt)[1] + line_gap
    return y


def paste_icon(base, xy, size):
    icon = Image.open(ICON).convert("RGBA").resize((size, size), Image.Resampling.LANCZOS)
    base.alpha_composite(icon, xy)


def pill(draw, xy, text, color, fnt=None, pad_x=20, pad_y=10, bg_alpha=24):
    fnt = fnt or font(28, bold=True)
    x, y = xy
    tw, th = text_size(draw, text, fnt)
    box = (x, y, x + tw + pad_x * 2, y + th + pad_y * 2)
    rounded_rect(draw, box, 999, rgba(color, bg_alpha), rgba(color, 70), 1)
    draw.text((x + pad_x, y + pad_y - 1), text, font=fnt, fill=color)
    return box[2], box[3]


def draw_kbd(draw, xy, text, scale=1.0):
    f = font(int(28 * scale), bold=True)
    x, y = xy
    tw, th = text_size(draw, text, f)
    box = (x, y, x + tw + int(34 * scale), y + th + int(22 * scale))
    rounded_rect(draw, box, int(12 * scale), WHITE, LINE, max(1, int(2 * scale)))
    draw.line((box[0] + 4, box[3] - 3, box[2] - 4, box[3] - 3), fill=(203, 213, 225), width=max(1, int(2 * scale)))
    draw.text((x + int(17 * scale), y + int(9 * scale)), text, font=f, fill=INK)
    return box


def draw_feature_icon(draw, cx, cy, color, kind):
    draw.rounded_rectangle((cx - 26, cy - 26, cx + 26, cy + 26), radius=14, fill=rgba(color, 24), outline=rgba(color, 64))
    if kind == "hotkey":
        draw.rectangle((cx - 12, cy - 7, cx + 12, cy + 9), outline=color, width=3)
        draw.line((cx - 6, cy + 2, cx + 6, cy + 2), fill=color, width=3)
    elif kind == "search":
        draw.ellipse((cx - 13, cy - 13, cx + 7, cy + 7), outline=color, width=4)
        draw.line((cx + 5, cy + 6, cx + 16, cy + 17), fill=color, width=4)
    elif kind == "image":
        draw.rectangle((cx - 14, cy - 12, cx + 14, cy + 12), outline=color, width=3)
        draw.polygon([(cx - 12, cy + 10), (cx - 3, cy), (cx + 4, cy + 6), (cx + 10, cy - 1), (cx + 14, cy + 10)], fill=rgba(color, 130))


def draw_history_mock(base, x, y, w, h):
    draw = ImageDraw.Draw(base)
    shadow(base, (x, y, x + w, y + h), radius=34, blur=42, alpha=54, offset=(0, 28))
    rounded_rect(draw, (x, y, x + w, y + h), 34, WHITE, rgba(LINE, 255), 2)
    rounded_rect(draw, (x, y, x + w, y + 82), 34, WHITE)
    draw.line((x, y + 82, x + w, y + 82), fill=LINE, width=2)
    paste_icon(base, (x + 32, y + 24), 34)
    draw.text((x + 78, y + 28), "XCopy", font=font(25, bold=True), fill=INK_2)
    center_text(draw, x + w // 2, y + 43, "剪贴板历史", font(23, bold=True), INK_2)
    for i, mark in enumerate(["-", "□", "×"]):
        bx = x + w - 144 + i * 42
        rounded_rect(draw, (bx, y + 24, bx + 30, y + 54), 9, (248, 250, 252), rgba(LINE, 200))
        center_text(draw, bx + 15, y + 38, mark, font(18), INK_3)

    body_y = y + 112
    rounded_rect(draw, (x + 34, body_y, x + w - 34, body_y + 62), 16, (248, 250, 252), rgba(LINE, 220))
    draw.text((x + 58, body_y + 15), "搜索剪贴板内容…", font=font(24), fill=(148, 163, 184))
    tab_y = body_y + 90
    tabs = [("全部 128", BLUE), ("文本", BLUE), ("链接", GREEN), ("图片", PURPLE)]
    tx = x + 34
    for i, (label, color) in enumerate(tabs):
        tw, th = text_size(draw, label, font(21, bold=True))
        rounded_rect(draw, (tx, tab_y, tx + tw + 30, tab_y + 44), 13, WHITE if i == 0 else (248, 250, 252), rgba(color, 80 if i == 0 else 30))
        draw.text((tx + 15, tab_y + 9), label, font=font(21, bold=True), fill=color if i == 0 else INK_3)
        tx += tw + 42

    items = [
        ("文本", BLUE, "会议纪要：下周发布 XCopy v0.2", "Word", "刚刚"),
        ("链接", GREEN, "https://github.com/yingjunnan/XCopy", "Chrome", "2 分钟前"),
        ("图片", PURPLE, "截图已保存，点击可查看大图", "SnippingTool", "5 分钟前"),
        ("文本", BLUE, "安装后常驻托盘，后台自动记录", "Notepad", "8 分钟前"),
    ]
    iy = tab_y + 70
    for idx, (kind, color, content, app, time) in enumerate(items):
        item_h = 122 if kind != "图片" else 150
        rounded_rect(draw, (x + 34, iy, x + w - 34, iy + item_h), 22, WHITE, rgba((15, 23, 42), 18))
        draw.ellipse((x + 58, iy + 28, x + 70, iy + 40), fill=color)
        draw.text((x + 84, iy + 20), kind, font=font(20, bold=True), fill=color)
        draw.text((x + 150, iy + 22), app, font=font(18), fill=(148, 163, 184))
        draw.text((x + w - 154, iy + 22), time, font=font(18), fill=(148, 163, 184))
        if kind == "图片":
            rounded_rect(draw, (x + 58, iy + 64, x + 222, iy + 124), 14, rgba(PURPLE, 32), rgba(PURPLE, 60))
            draw_feature_icon(draw, x + 100, iy + 94, PURPLE, "image")
            draw.text((x + 244, iy + 73), content, font=font(24), fill=INK_2)
        else:
            draw_wrapped(draw, (x + 58, iy + 63), content, font(25), INK, w - 140, 5)
        iy += item_h + 16


def draw_quick_panel(base, x, y, w, h):
    draw = ImageDraw.Draw(base)
    shadow(base, (x, y, x + w, y + h), radius=32, blur=40, alpha=72, offset=(0, 22))
    rounded_rect(draw, (x, y, x + w, y + h), 32, WHITE, rgba((148, 163, 184), 120), 2)
    rounded_rect(draw, (x + 24, y + 24, x + w - 24, y + 82), 16, (248, 250, 252), rgba(LINE, 220))
    draw.text((x + 48, y + 39), "搜索并直接粘贴…", font=font(24), fill=(148, 163, 184))
    entries = [
        ("刚复制的登录验证码", "Chrome"),
        ("客户邮件回复模板", "Outlook"),
        ("https://xcopy.debugmy.com", "Edge"),
    ]
    iy = y + 106
    for i, (content, app) in enumerate(entries):
        fill = rgba(BLUE, 18) if i == 0 else (255, 255, 255, 255)
        outline = rgba(BLUE, 100) if i == 0 else rgba((15, 23, 42), 16)
        rounded_rect(draw, (x + 24, iy, x + w - 24, iy + 82), 18, fill, outline, 2)
        draw.text((x + 50, iy + 18), content, font=font(25, bold=i == 0), fill=INK)
        draw.text((x + 50, iy + 50), app, font=font(18), fill=INK_3)
        if i == 0:
            draw.text((x + w - 112, iy + 28), "Enter", font=font(20, bold=True), fill=BLUE)
        iy += 96
    draw.text((x + 36, y + h - 44), "↑↓ 选择 · Enter 粘贴 · Esc 关闭", font=font(19), fill=INK_3)


def poster_one():
    base = gradient((W, H), (255, 255, 255, 255), (237, 246, 255, 255))
    draw = ImageDraw.Draw(base)

    # Subtle Windows-like background bands.
    for i in range(6):
        y = 220 + i * 260
        draw.line((60, y, W - 60, y + 120), fill=rgba(BLUE, 10), width=3)
    draw.rectangle((0, H - 420, W, H), fill=(248, 250, 252))

    paste_icon(base, (90, 92), 74)
    draw.text((182, 108), "XCopy", font=font(45, bold=True), fill=INK)
    pill(draw, (740, 104), "Windows 10 / 11", BLUE, font(22, bold=True), 18, 9, 20)

    draw.text((90, 250), "剪贴板，", font=font(78, bold=True), fill=INK)
    draw.text((90, 344), "从未如此顺手", font=font(78, bold=True), fill=BLUE)
    draw_wrapped(
        draw,
        (92, 470),
        "自动记录文本、链接与图片，按快捷键即时唤起，秒搜分类，复制粘贴不再来回切窗口。",
        font(32),
        INK_2,
        850,
        14,
    )

    px, py = 92, 620
    px, _ = pill(draw, (px, py), "Ctrl+Shift+V 唤起", BLUE, font(24, bold=True), 18, 9, 24)
    px, _ = pill(draw, (px + 14, py), "秒搜分类", GREEN, font(24, bold=True), 18, 9, 24)
    pill(draw, (px + 14, py), "本地存储", PURPLE, font(24, bold=True), 18, 9, 24)

    draw_history_mock(base, 90, 760, 900, 850)

    center_text(draw, W // 2, 1738, "免费 · 开源 · 轻量 Windows 剪贴板历史工具", font(29, bold=True), INK)
    center_text(draw, W // 2, 1800, "xcopy.debugmy.com", font(25), BLUE_DARK)
    return base.convert("RGB")


def poster_two():
    base = gradient((W, H), (20, 32, 56, 255), (247, 250, 255, 255))
    draw = ImageDraw.Draw(base)

    # Abstract desktop surface, kept clean for mobile readability.
    draw.rounded_rectangle((-90, 1110, W + 90, H + 140), radius=80, fill=(248, 250, 252))
    draw.rounded_rectangle((84, 1420, 996, 1748), radius=42, fill=(226, 232, 240))
    draw.rounded_rectangle((142, 1464, 938, 1718), radius=34, fill=WHITE, outline=(203, 213, 225), width=2)

    paste_icon(base, (86, 88), 66)
    draw.text((170, 103), "XCopy", font=font(42, bold=True), fill=WHITE)
    pill(draw, (720, 100), "快速粘贴", BLUE, font(23, bold=True), 18, 9, 36)

    draw.text((84, 238), "双击 Ctrl，", font=font(78, bold=True), fill=WHITE)
    draw.text((84, 332), "秒粘贴", font=font(98, bold=True), fill=(147, 197, 253))
    draw_wrapped(
        draw,
        (88, 484),
        "鼠标处弹出轻量面板，选中一条历史，内容直接粘贴到当前光标。",
        font(33),
        (226, 232, 240),
        830,
        14,
    )

    # Big key press cue.
    shadow(base, (146, 662, 934, 838), radius=42, blur=38, alpha=92, offset=(0, 26))
    rounded_rect(draw, (146, 662, 934, 838), 42, rgba(WHITE, 245), rgba((191, 219, 254), 180), 2)
    draw_kbd(draw, (218, 714), "Ctrl", 1.55)
    draw.text((430, 730), "× 2", font=font(58, bold=True), fill=BLUE)
    draw.text((566, 730), "唤出", font=font(52, bold=True), fill=INK)

    # Cursor target and panel.
    draw.line((668, 955, 668, 1238), fill=rgba(BLUE, 180), width=5)
    draw.polygon([(650, 955), (705, 982), (675, 994), (700, 1040), (676, 1052), (650, 1007)], fill=BLUE)
    draw_quick_panel(base, 154, 910, 772, 458)

    points = [
        ("无需来回切窗口", BLUE, "hotkey"),
        ("↑↓ 选择，Enter 粘贴", GREEN, "search"),
        ("SQLite 本地存储更安心", PURPLE, "image"),
    ]
    y = 1496
    for label, color, kind in points:
        draw_feature_icon(draw, 184, y + 30, color, kind)
        draw.text((238, y + 11), label, font=font(28, bold=True), fill=INK)
        y += 70

    center_text(draw, W // 2, 1792, "XCopy 让每一次复制都更顺手", font(32, bold=True), INK)
    center_text(draw, W // 2, 1848, "免费开源 · Windows 原生", font(25), INK_2)
    return base.convert("RGB")


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    files = [
        (OUT / "xcopy-mobile-promo-01.png", poster_one()),
        (OUT / "xcopy-mobile-promo-02.png", poster_two()),
    ]
    for path, img in files:
        img.save(path, "PNG", optimize=True)
        print(f"{path} {img.size[0]}x{img.size[1]}")


if __name__ == "__main__":
    main()
