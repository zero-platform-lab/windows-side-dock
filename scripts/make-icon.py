"""Dockのアイコン（黒い角丸に銀のバー）を描き、assets/ に ico、RGBA、ロゴPNGを書き出す。

使い方: python scripts/make-icon.py  （Pillowが必要）
"""
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
ASSETS = ROOT / "assets"
# 64単位の設計図を、この倍率で大きく描いてから縮小して滑らかにする。
SCALE = 16
ICO_SIZES = [16, 20, 24, 32, 40, 48, 64, 128, 256]
WINDOW_ICON_SIZE = 64


def gradient(size, stops):
    """上から下へのグラデーション画像。stops は (位置0〜1, (r, g, b)) の並び。"""
    width, height = size
    image = Image.new("RGBA", size)
    draw = ImageDraw.Draw(image)
    for y in range(height):
        t = y / max(height - 1, 1)
        for (p0, c0), (p1, c1) in zip(stops, stops[1:]):
            if p0 <= t <= p1:
                k = (t - p0) / (p1 - p0)
                color = tuple(round(a + (b - a) * k) for a, b in zip(c0, c1))
                break
        draw.line([(0, y), (width, y)], fill=color + (255,))
    return image


def paste_shape(canvas, box, radius, fill_image):
    """box の角丸四角形の形で fill_image を貼る。座標は64単位。"""
    x0, y0, x1, y1 = (round(v * SCALE) for v in box)
    mask = Image.new("L", canvas.size, 0)
    ImageDraw.Draw(mask).rounded_rectangle((x0, y0, x1, y1), round(radius * SCALE), fill=255)
    layer = Image.new("RGBA", canvas.size, (0, 0, 0, 0))
    layer.paste(fill_image.resize((x1 - x0, y1 - y0)), (x0, y0))
    canvas.paste(layer, (0, 0), mask)


def draw_master():
    size = 64 * SCALE
    canvas = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    metal = gradient((64, 64), [(0, (74, 79, 88)), (0.45, (35, 38, 44)), (1, (13, 14, 17))])
    silver = gradient((64, 64), [(0, (244, 246, 250)), (0.55, (185, 192, 204)), (1, (138, 146, 160))])
    # 縁取りの色で一回り大きく描き、その内側に本体を重ねる。
    paste_shape(canvas, (3.25, 3.25, 60.75, 60.75), 13.75, Image.new("RGBA", (1, 1), (125, 133, 147, 255)))
    paste_shape(canvas, (4.75, 4.75, 59.25, 59.25), 12.25, metal)
    paste_shape(canvas, (40, 10, 52, 54), 6, silver)
    draw = ImageDraw.Draw(canvas)
    for y, width in [(20, 18), (30, 22), (40, 14)]:
        draw.rounded_rectangle(
            (13 * SCALE, y * SCALE, (13 + width) * SCALE, (y + 3.5) * SCALE),
            round(1.75 * SCALE),
            fill=(107, 114, 128, 255),
        )
    r = 3 * SCALE
    draw.ellipse((46 * SCALE - r, 32 * SCALE - r, 46 * SCALE + r, 32 * SCALE + r), fill=(60, 208, 112, 255))
    return canvas


def main():
    ASSETS.mkdir(exist_ok=True)
    master = draw_master()
    images = [master.resize((s, s), Image.LANCZOS) for s in ICO_SIZES]
    images[-1].save(ASSETS / "icon.ico", sizes=[(s, s) for s in ICO_SIZES], append_images=images[:-1])
    images[-1].save(ASSETS / "icon-256.png")
    window = master.resize((WINDOW_ICON_SIZE, WINDOW_ICON_SIZE), Image.LANCZOS)
    (ASSETS / "icon-64.rgba").write_bytes(window.tobytes())
    # 右クリックメニュー用スパースパッケージ（installer/sparse）のロゴ。
    for size in (44, 150):
        master.resize((size, size), Image.LANCZOS).save(ASSETS / f"logo-{size}.png")


if __name__ == "__main__":
    main()
