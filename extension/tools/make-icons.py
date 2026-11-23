"""Extension icons: the same tile as the app (tools/make_icon.py), at the sizes browsers ask for."""
import importlib.util
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("make_icon", os.path.join(HERE, "..", "..", "tools", "make_icon.py"))
make_icon = importlib.util.module_from_spec(spec)
spec.loader.exec_module(make_icon)

SIZES = (16, 32, 48, 128)


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else os.path.join(HERE, "..", "src", "icons")
    os.makedirs(out, exist_ok=True)
    for size in SIZES:
        make_icon.render(size).save(os.path.join(out, f"icon-{size}.png"))
    with open(os.path.join(out, "icon.svg"), "w", encoding="utf-8") as f:
        f.write(make_icon.svg())


main()
