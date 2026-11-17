"""Draw the sendingUnicorns app icon: a unicorn resting a hoof on a rocket taking off.

Same unicorn as the codingUnicorns / storingUnicorns / browsingUnicorns icons, only the
prop changes.

Rendered at 4x (4096 px) with plain polygons, then downsampled for antialiasing.
Requires Pillow. Regenerate the app icon with:

    python scripts/draw-icon.py assets/icon.png 256

assets/icon.ico is derived from it by scripts/New-Icon.ps1.
"""
import math
import sys
from PIL import Image, ImageDraw, ImageFilter

S = 4096          # supersampled canvas
U = S / 1024      # design units are on a 1024 grid


def p(x, y):
    return (x * U, y * U)


def bezier(p0, p1, p2, p3, n=40):
    pts = []
    for i in range(n + 1):
        t = i / n
        mt = 1 - t
        x = mt**3 * p0[0] + 3 * mt**2 * t * p1[0] + 3 * mt * t**2 * p2[0] + t**3 * p3[0]
        y = mt**3 * p0[1] + 3 * mt**2 * t * p1[1] + 3 * mt * t**2 * p2[1] + t**3 * p3[1]
        pts.append(p(x, y))
    return pts


def path(start, *curves):
    """start=(x,y); each curve = (c1, c2, end) cubic, or (end,) straight line."""
    pts = [p(*start)]
    cur = start
    for c in curves:
        if len(c) == 1:
            pts.append(p(*c[0]))
            cur = c[0]
        else:
            pts.extend(bezier(cur, c[0], c[1], c[2])[1:])
            cur = c[2]
    return pts


def rounded_rect(d, box, r, fill):
    x0, y0, x1, y1 = box
    d.rounded_rectangle((x0 * U, y0 * U, x1 * U, y1 * U), radius=r * U, fill=fill)


def vgradient(size, top, bottom):
    w, h = size
    g = Image.new("RGBA", size)
    gd = ImageDraw.Draw(g)
    for y in range(h):
        t = y / max(h - 1, 1)
        c = tuple(int(top[i] + (bottom[i] - top[i]) * t) for i in range(4))
        gd.line([(0, y), (w, y)], fill=c)
    return g


def draw():
    img = Image.new("RGBA", (S, S), (0, 0, 0, 0))

    # ── Background: rounded square, deep night purple gradient ───────────
    bg = vgradient((S, S), (58, 36, 110, 255), (22, 18, 48, 255))
    mask = Image.new("L", (S, S), 0)
    ImageDraw.Draw(mask).rounded_rectangle((24 * U, 24 * U, 1000 * U, 1000 * U), radius=210 * U, fill=255)
    img.paste(bg, (0, 0), mask)
    d = ImageDraw.Draw(img)

    # Sparkles in the sky
    for (x, y, r) in [(170, 170, 16), (860, 150, 22), (930, 330, 12), (120, 430, 10), (700, 110, 9)]:
        star = [p(x, y - r * 2.2), p(x + r * 0.5, y - r * 0.5), p(x + r * 2.2, y), p(x + r * 0.5, y + r * 0.5),
                p(x, y + r * 2.2), p(x - r * 0.5, y + r * 0.5), p(x - r * 2.2, y), p(x - r * 0.5, y - r * 0.5)]
        d.polygon(star, fill=(255, 236, 160, 255))

    # ── Rainbow mane (behind the neck): outer stripe first so every band shows ──
    rainbow = [(255, 89, 94), (255, 146, 76), (255, 202, 58), (138, 201, 38), (25, 170, 230), (106, 76, 255)]
    for i, col in reversed(list(enumerate(rainbow))):
        o = i * 26  # outward offset: red innermost, purple outermost
        mane = path(
            (370, 280),
            ((300 - o, 260 - o * 0.3), (250 - o, 360), (265 - o, 470)),
            ((280 - o, 560), (190 - o, 600), (200 - o, 700)),
            ((210 - o, 800), (150 - o, 860), (170 - o, 1010)),
            ((330, 1010),),
            ((330, 700), (330, 450), (370, 280)),
        )
        d.polygon(mane, fill=col + (255,))

    # ── Head + neck (white horse profile, looking down-right at the laptop) ──
    head = path(
        (270, 1010),
        ((270, 760), (280, 520), (350, 330)),      # back of the neck up to the poll
        ((360, 280), (372, 220), (388, 165)),      # ear, back edge
        ((425, 215), (445, 255), (452, 292)),      # ear, front edge
        ((540, 290), (620, 330), (690, 420)),      # forehead to nose bridge
        ((730, 470), (775, 520), (778, 568)),      # nose
        ((780, 615), (745, 640), (700, 642)),      # muzzle tip / upper lip
        ((665, 645), (640, 660), (605, 655)),      # chin
        ((560, 648), (520, 610), (505, 560)),      # big round jowl back to the jaw corner
        ((470, 620), (470, 720), (490, 800)),      # throat
        ((505, 870), (520, 950), (525, 1010)),     # front of the neck
    )
    d.polygon(head, fill=(250, 248, 255, 255))
    # Jowl shading so the cheek reads as a separate volume
    jowl = path((505, 560), ((520, 610), (560, 640), (605, 650)), ((575, 600), (545, 575), (505, 560)))
    d.polygon(jowl, fill=(228, 222, 246, 255))
    neck_shade = path((300, 1010), ((300, 820), (330, 680), (400, 590)), ((430, 720), (445, 880), (450, 1010)))
    d.polygon(neck_shade, fill=(234, 229, 250, 255))
    # Inner ear
    ear = path((388, 195), ((405, 228), (420, 255), (428, 282)), ((398, 285),), ((392, 250), (388, 225), (388, 195)))
    d.polygon(ear, fill=(255, 182, 213, 255))

    # ── Horn: golden spiral ──────────────────────────────────────────────
    base_l, base_r, tip = (455, 292), (515, 305), (640, 60)
    d.polygon([p(*base_l), p(*tip), p(*base_r)], fill=(255, 206, 84, 255))
    for k in range(1, 6):
        t = k / 6.2
        lx = base_l[0] + (tip[0] - base_l[0]) * t
        ly = base_l[1] + (tip[1] - base_l[1]) * t
        rx = base_r[0] + (tip[0] - base_r[0]) * t
        ry = base_r[1] + (tip[1] - base_r[1]) * t
        d.line([p(lx, ly), p(rx, ry - 16 * (1 - t))], fill=(214, 150, 40, 255), width=int(9 * U * (1 - t * 0.5)))
    d.polygon([p(472, 278), p(622, 88), p(482, 296)], fill=(255, 240, 180, 255))

    # Forelock: three rainbow strands over the forehead, in front of the horn base
    for i, col in enumerate([(255, 89, 94), (255, 202, 58), (25, 170, 230)]):
        o = i * 26
        lock = path((410 + o, 280), ((470 + o, 285), (500 + o, 330), (480 + o, 380)),
                    ((465 + o, 345), (440 + o, 320), (410 + o, 280)))
        d.polygon(lock, fill=col + (255,))

    # ── Face ─────────────────────────────────────────────────────────────
    # Happy closed eye looking down at the screen, with lashes
    d.arc((545 * U, 395 * U, 615 * U, 455 * U), start=200, end=340, fill=(40, 28, 70, 255), width=int(11 * U))
    for ang in (215, 240, 265):
        a = math.radians(ang)
        cx, cy, rr = 580, 425, 35
        x0, y0 = cx + rr * math.cos(a), cy + rr * math.sin(a)
        d.line([p(x0, y0), p(x0 + 16 * math.cos(a), y0 + 16 * math.sin(a))], fill=(40, 28, 70, 255), width=int(8 * U))
    d.ellipse((595 * U, 480 * U, 660 * U, 520 * U), fill=(255, 160, 200, 255))      # blush
    d.ellipse((738 * U, 575 * U, 756 * U, 597 * U), fill=(196, 176, 228, 255))      # nostril
    d.arc((695 * U, 605 * U, 745 * U, 640 * U), start=30, end=150, fill=(40, 28, 70, 255), width=int(7 * U))

    # ── Rocket: tilted up-right, glowing, flame trailing down-left ─────────
    cx, cy, tilt, k = 780, 812, math.radians(38), 1.12   # centre, lean, scale
    cos_t, sin_t = math.cos(tilt), math.sin(tilt)

    def r(x, y):
        """A point of the upright rocket (origin at its centre), tilted and placed."""
        x, y = x * k, y * k
        return ((cx + x * cos_t - y * sin_t) * U, (cy + x * sin_t + y * cos_t) * U)

    def outline(*pts):
        return [r(x, y) for x, y in pts]

    def curve(p0, p1, p2, p3, n=24):
        return [r(*bezier_pt(p0, p1, p2, p3, i / n)) for i in range(n + 1)]

    def bezier_pt(p0, p1, p2, p3, t):
        mt = 1 - t
        return (mt ** 3 * p0[0] + 3 * mt * mt * t * p1[0] + 3 * mt * t * t * p2[0] + t ** 3 * p3[0],
                mt ** 3 * p0[1] + 3 * mt * mt * t * p1[1] + 3 * mt * t * t * p2[1] + t ** 3 * p3[1])

    rim = (90, 220, 255, 255)
    body_pts = (curve((0, -215), (55, -170), (88, -90), (88, -20))
                + outline((88, 120), (70, 150), (-70, 150), (-88, 120))
                + curve((-88, -20), (-88, -90), (-55, -170), (0, -215)))

    # Glow behind everything
    glow = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    ImageDraw.Draw(glow).polygon(body_pts, fill=(90, 220, 255, 150))
    glow = glow.filter(ImageFilter.GaussianBlur(45 * U))
    img.alpha_composite(glow)

    # Flame: three nested teardrops, hottest inside
    flame = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    fd = ImageDraw.Draw(flame)
    for width, length, col in ((62, 330, (255, 121, 80, 255)), (44, 280, (255, 184, 108, 255)), (24, 225, (255, 240, 170, 255))):
        fd.polygon(curve((-width, 150), (-width, 210), (-width * 0.4, length - 40), (0, length))
                   + curve((0, length), (width * 0.4, length - 40), (width, 210), (width, 150)), fill=col)
    img.alpha_composite(flame.filter(ImageFilter.GaussianBlur(3 * U)))
    d = ImageDraw.Draw(img)

    # Fins, behind the body
    fin = (255, 121, 198, 255)
    d.polygon(curve((-80, 30), (-130, 70), (-150, 140), (-140, 205)) + outline((-70, 150)), fill=fin)
    d.polygon(curve((80, 30), (130, 70), (150, 140), (140, 205)) + outline((70, 150)), fill=fin)
    d.polygon(outline((-10, 60), (10, 60), (10, 200), (-10, 200)), fill=(220, 80, 165, 255))

    # Body: light on the left, shaded on the right
    d.polygon(body_pts, fill=(236, 232, 252, 255))
    d.polygon(curve((0, -215), (55, -170), (88, -90), (88, -20)) + outline((88, 120), (70, 150), (30, 150))
              + curve((30, 150), (40, 0), (35, -120), (0, -215)), fill=(196, 188, 236, 255))
    # Nose cone and the band at the base
    d.polygon(curve((0, -215), (40, -183), (60, -150), (66, -125)) + curve((-66, -125), (-60, -150), (-40, -183), (0, -215)),
              fill=(255, 121, 198, 255))
    d.polygon(outline((-88, 100), (88, 100), (88, 120), (70, 150), (-70, 150), (-88, 120)), fill=(128, 138, 210, 255))
    d.polygon(body_pts, outline=rim, width=int(8 * U))

    # Porthole
    px, py = r(0, -35)
    pr = 40 * k * U
    d.ellipse((px - pr, py - pr, px + pr, py + pr), fill=(66, 72, 128, 255), outline=rim, width=int(9 * U))
    d.ellipse((px - pr * 0.62, py - pr * 0.62, px + pr * 0.62, py + pr * 0.62), fill=(90, 160, 235, 255))
    d.arc((px - pr * 0.5, py - pr * 0.5, px + pr * 0.5, py + pr * 0.5), start=190, end=260,
          fill=(220, 240, 255, 255), width=int(7 * U))

    # Speed lines trailing beside the flame
    for (x0, y0), (x1, y1) in [((-125, 215), (-125, 300)), ((125, 215), (125, 280)), ((165, 120), (165, 175))]:
        d.line([r(x0, y0), r(x1, y1)], fill=(150, 235, 255, 255), width=int(8 * U))

    # ── Foreleg resting on the rocket, hoof on its side ─────────────
    leg = path((345, 1010), ((380, 905), (490, 770), (628, 712)),
               ((664, 738),), ((545, 800), (465, 905), (440, 1010)))
    d.polygon(leg, fill=(250, 248, 255, 255))
    d.polygon(path((360, 1010), ((395, 915), (485, 800), (590, 745)), ((520, 820), (455, 915), (432, 1010))),
              fill=(228, 222, 246, 255))  # leg shading
    hoof = path((618, 700), ((655, 676), (705, 684), (718, 712)), ((696, 744),), ((658, 748), (630, 732), (618, 700)))
    d.polygon(hoof, fill=(120, 96, 170, 255))
    d.line([p(632, 708), p(700, 702)], fill=(160, 136, 210, 255), width=int(8 * U))  # hoof highlight

    # Clip everything to the rounded square
    out = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    out.paste(img, (0, 0), mask)
    return out


if __name__ == "__main__":
    dest = sys.argv[1]
    size = int(sys.argv[2]) if len(sys.argv) > 2 else 1024
    draw().resize((size, size), Image.LANCZOS).save(dest)
