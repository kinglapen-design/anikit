# 新建文档占位模板生成（2026-08-15，3.1 二轮）：用户真实空白文件到位前的临时占位，
# 到位后直接覆盖 assets/templates/ 同名文件即可（本脚本产物不再使用）。
# docx/xlsx = 最小 OOXML zip；psd = 1x1 RGB 最小 PSD；ai = 单页最小 PDF（Illustrator 兼容 PDF）；
# fla = CS5.5+ zip 容器最小 DOMDocument（Animate 可能报修复，占位性质）。
# 用法：python tools/gen-doc-templates.py
import struct
import zipfile
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "assets" / "templates"
OUT.mkdir(parents=True, exist_ok=True)


def writetext(name: str, text: str) -> None:
    (OUT / name).write_text(text, encoding="utf-8", newline="\n")
    print("写出", name)


def writezip(name: str, entries: dict[str, str]) -> None:
    with zipfile.ZipFile(OUT / name, "w", zipfile.ZIP_DEFLATED) as z:
        for path, content in entries.items():
            z.writestr(path, content)
    print("写出", name)


# ---- blank.docx（最小 OOXML 文字文档）----
writezip("blank.docx", {
    "[Content_Types].xml": """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>""",
    "_rels/.rels": """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>""",
    "word/document.xml": """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p/></w:body></w:document>""",
})

# ---- blank.xlsx（最小 OOXML 表格，单空表 Sheet1）----
writezip("blank.xlsx", {
    "[Content_Types].xml": """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/></Types>""",
    "_rels/.rels": """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>""",
    "xl/workbook.xml": """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets></workbook>""",
    "xl/_rels/workbook.xml.rels": """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>""",
    "xl/worksheets/sheet1.xml": """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData/></worksheet>""",
})

# ---- blank.psd（1x1 RGB 8bit 最小 PSD，无图层）----
psd = b"8BPS" + struct.pack(">H", 1) + b"\0" * 6 + struct.pack(">HIIHH", 3, 1, 1, 8, 3)
psd += struct.pack(">I", 0)  # 颜色模式数据段
psd += struct.pack(">I", 0)  # 图像资源段
psd += struct.pack(">I", 0)  # 图层与蒙版段
psd += struct.pack(">H", 0) + b"\xff\xff\xff"  #  raw 压缩 + 1 像素三通道（白）
(OUT / "blank.psd").write_bytes(psd)
print("写出 blank.psd")

# ---- blank.ai（单页最小 PDF，Illustrator 自 9.0 起 .ai 即 PDF 兼容格式）----
objects = [
    b"<< /Type /Catalog /Pages 2 0 R >>",
    b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 1920 1080] >>",
]
pdf = bytearray(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n")
offsets = []
for i, body in enumerate(objects, start=1):
    offsets.append(len(pdf))
    pdf += f"{i} 0 obj\n".encode() + body + b"\nendobj\n"
xref_pos = len(pdf)
pdf += f"xref\n0 {len(objects) + 1}\n0000000000 65535 f \n".encode()
for off in offsets:
    pdf += f"{off:010d} 00000 n \n".encode()
pdf += f"trailer\n<< /Size {len(objects) + 1} /Root 1 0 R >>\nstartxref\n{xref_pos}\n%%EOF\n".encode()
(OUT / "blank.ai").write_bytes(bytes(pdf))
print("写出 blank.ai")

# ---- blank.fla（CS5.5+ zip 容器最小 DOMDocument，占位——Animate 可能报修复，等用户真实文件）----
writezip("blank.fla", {
    "Document.xml": """<?xml version="1.0" encoding="UTF-8"?>
<DOMDocument xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns="http://ns.adobe.com/xfl/2008/" currentTimeline="1" xflVersion="2.971" majorVersion="24" buildNumber="1">
  <folders/>
  <media/>
  <symbols/>
  <timelines>
    <DOMTimeline name="Scene 1">
      <layers>
        <DOMLayer name="Layer_1">
          <frames><DOMFrame index="0" duration="1"><elements/></DOMFrame></frames>
        </DOMLayer>
      </layers>
    </DOMTimeline>
  </timelines>
</DOMDocument>""",
    "mimetype": "application/vnd.adobe.fla",
})
