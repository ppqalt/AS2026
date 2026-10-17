# Synteettisten testitiedostojen generaattori; ei sovelluksen ajonaikainen riippuvuus.
from pathlib import Path
from zipfile import ZipFile, ZIP_DEFLATED
from xml.sax.saxutils import escape
import xlwt
out=Path(__file__).parent
sheets={
'RATA':[
['Luokka','C','','','','Luokka','B'],
[],
['EMIT no','Asemanimi','Ihanneaika','kdi','Jakso','EMIT no','Asemanimi','Ihannea','kdi','Jakso'],
[0,'Lähtö',0,0,'',0,'Lähtö',0,0,''],
[140,'JAT1',7.28,1,15,0,'AT1','05:00',2,10],
[0,'AT2','12,34',0,15,100,'Maali','10:00',0,10],
[100,'Maali',13,0,15],
],
'KILP':[
['nro','Lka','EMIT','L-aika','','Kuski','Seura','Kartturi','Seura','Auto'],
[1,'C',12345,17,2,'Mallikuljettaja Ää','Seura A','Mallikartanlukija','Seura B','Malliauto'],
[2,'B',23456,'18:30:05','','Toinen kuljettaja','Seura C','Toinen kartanlukija','Seura D','Toinen auto'],
],
'ÄLÄ_TUO':[['Tätä välilehteä ei tuoda'],[999,'Tuntematon',99999]],
}
book=xlwt.Workbook()
for name,rows in sheets.items():
    s=book.add_sheet(name)
    for r,row in enumerate(rows):
        for c,value in enumerate(row):
            if value!='':s.write(r,c,value)
book.save(str(out/'synteettinen.xls'))

def col(i):
    s='';i+=1
    while i:i,a=divmod(i-1,26);s=chr(65+a)+s
    return s
with ZipFile(out/'synteettinen.xlsx','w',ZIP_DEFLATED) as z:
    z.writestr('[Content_Types].xml','<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>'+''.join(f'<Override PartName="/xl/worksheets/sheet{i}.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>' for i in range(1,len(sheets)+1))+'</Types>')
    z.writestr('_rels/.rels','<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>')
    z.writestr('xl/workbook.xml','<?xml version="1.0" encoding="UTF-8"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets>'+''.join(f'<sheet name="{name}" sheetId="{i}" r:id="rId{i}"/>' for i,name in enumerate(sheets,1))+'</sheets></workbook>')
    z.writestr('xl/_rels/workbook.xml.rels','<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'+''.join(f'<Relationship Id="rId{i}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet{i}.xml"/>' for i in range(1,len(sheets)+1))+'</Relationships>')
    for i,(name,rows) in enumerate(sheets.items(),1):
        xml='<?xml version="1.0" encoding="UTF-8"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>'
        for r,row in enumerate(rows,1):
            xml+=f'<row r="{r}">'
            for c,value in enumerate(row):
                if value=='':continue
                ref=f'{col(c)}{r}'
                xml+=f'<c r="{ref}"><v>{value}</v></c>' if isinstance(value,(int,float)) else f'<c r="{ref}" t="inlineStr"><is><t>{escape(value)}</t></is></c>'
            xml+='</row>'
        xml+='</sheetData></worksheet>'
        z.writestr(f'xl/worksheets/sheet{i}.xml',xml)
