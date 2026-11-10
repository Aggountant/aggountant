#!/usr/bin/env python3
"""Render an aggountant monthly-statements native JSON payload as an ordinary PDF."""

from __future__ import annotations

import argparse
import html
import json
import os
import sys
from pathlib import Path


def fail(message: str, code: int = 1) -> "None":
    print(f"render-statement-pdf: {message}", file=sys.stderr)
    raise SystemExit(code)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Render native monthly statement JSON as PDF")
    parser.add_argument("input", type=Path, help="monthly-statements native JSON")
    parser.add_argument("output", type=Path, help="new PDF path (never overwritten)")
    parser.add_argument("--font", type=Path, help="explicit TrueType font with Finnish glyphs")
    return parser.parse_args()


def load_report(path: Path) -> dict:
    try:
        raw = path.read_bytes()
    except FileNotFoundError:
        fail(f"input does not exist: {path}")
    except OSError as error:
        fail(f"could not read input {path}: {error}", 2)
    try:
        report = json.loads(raw)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"input is not valid UTF-8 JSON: {error}")
    required = {"schema_version", "report_type", "company", "period", "statement_date", "currency", "status", "completeness", "provenance", "sources", "sections"}
    if not isinstance(report, dict) or not required.issubset(report):
        fail(f"input is not a monthly-statements native payload; missing {sorted(required - set(report if isinstance(report, dict) else {}))}")
    if report["schema_version"] != 1 or report["report_type"] != "monthly_statements":
        fail(f"unsupported report schema/type: {report.get('schema_version')!r}/{report.get('report_type')!r}")
    for section in ("month_income", "year_to_date_income", "balance"):
        if section not in report["sections"]:
            fail(f"input is missing sections.{section}")
    return report


def choose_font(explicit: Path | None) -> Path:
    candidates = ([explicit] if explicit else []) + [
        Path("/System/Library/Fonts/Supplemental/Arial.ttf"),
        Path("/Library/Fonts/Arial.ttf"),
        Path("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"),
        Path("/usr/share/fonts/truetype/liberation2/LiberationSans-Regular.ttf"),
    ]
    for candidate in candidates:
        if candidate and candidate.is_file():
            return candidate
    if explicit:
        fail(f"font does not exist: {explicit}")
    fail("no Unicode TrueType font found; pass --font /path/to/font.ttf", 2)


def fi_amount(value: object) -> str:
    raw = str(value)
    negative = raw.startswith("-")
    if negative:
        raw = raw[1:]
    whole, dot, fraction = raw.partition(".")
    groups = []
    while whole:
        groups.append(whole[-3:])
        whole = whole[:-3]
    rendered = " ".join(reversed(groups or ["0"])) + "," + (fraction if dot else "00")
    return ("−" if negative else "") + rendered


def paragraph_text(value: object) -> str:
    return html.escape(str(value), quote=True)


def render(report: dict, output: Path, font_path: Path) -> None:
    try:
        from reportlab.lib import colors
        from reportlab.lib.enums import TA_RIGHT
        from reportlab.lib.pagesizes import A4
        from reportlab.lib.styles import ParagraphStyle, getSampleStyleSheet
        from reportlab.lib.units import mm
        from reportlab.pdfbase import pdfmetrics
        from reportlab.pdfbase.ttfonts import TTFont
        from reportlab.platypus import KeepTogether, PageBreak, Paragraph, SimpleDocTemplate, Spacer, Table, TableStyle
    except ImportError as error:
        fail(f"missing dependency reportlab ({error}); use a Python environment where reportlab is already installed", 2)

    font_name = "AggountantUnicode"
    try:
        pdfmetrics.registerFont(TTFont(font_name, str(font_path)))
    except Exception as error:
        fail(f"could not load Unicode font {font_path}: {error}", 2)

    styles = getSampleStyleSheet()
    for style in styles.byName.values():
        style.fontName = font_name
    styles.add(ParagraphStyle(name="Right", parent=styles["BodyText"], alignment=TA_RIGHT, fontName=font_name))
    styles.add(ParagraphStyle(name="Small", parent=styles["BodyText"], fontSize=7.5, leading=9.5, fontName=font_name))
    styles.add(ParagraphStyle(name="Notice", parent=styles["BodyText"], backColor=colors.HexColor("#fff3d6"), borderPadding=7, leading=13, fontName=font_name))

    output.parent.mkdir(parents=True, exist_ok=True)
    try:
        reservation = output.open("xb")
        reservation.close()
    except FileExistsError:
        fail(f"refusing to overwrite existing output: {output}")
    except OSError as error:
        fail(f"could not create output {output}: {error}", 2)

    currency = paragraph_text(report["currency"])
    story = []
    story.append(Paragraph(paragraph_text(report["company"]), styles["Title"]))
    story.append(Paragraph(f"Kuukausiraportti {paragraph_text(report['period'])}", styles["Heading2"]))
    story.append(Spacer(1, 4 * mm))

    provenance = report["provenance"]
    status = report["status"]
    metadata = [
        ("Tilannepäivä", report["statement_date"]),
        ("Esitystila", status["presentation"]),
        ("Ledger-kauden tila", status["ledger_period_state"]),
        ("ALV-ilmoitus kirjattu", "kyllä" if status["vat_filed"] else "ei"),
        ("Käyttäjän hyväksyntä", status["user_approval"]),
        ("Lakisääteinen tilinpäätös", "ei väitetä"),
        ("Valuutta", report["currency"]),
        ("Karttaversio", provenance["chart_version"]),
        ("Policy set", provenance["policy_set"]),
        ("KPA-policy", f"{provenance['kpa_policy_id']} @ {provenance['kpa_policy_version']}"),
        ("Lähdejoukon hash", provenance["source_set_hash"]),
        ("Tase-erittelyjen hash", provenance["tase_erittelyt_hash"]),
        ("Raporttiskeema", report["schema_version"]),
    ]
    table = Table([[Paragraph(paragraph_text(k), styles["Small"]), Paragraph(paragraph_text(v), styles["Small"])] for k, v in metadata], colWidths=[42 * mm, 133 * mm])
    table.setStyle(TableStyle([("GRID", (0, 0), (-1, -1), .25, colors.HexColor("#ccd4da")), ("VALIGN", (0, 0), (-1, -1), "TOP"), ("BACKGROUND", (0, 0), (0, -1), colors.HexColor("#eef2f4")), ("LEFTPADDING", (0, 0), (-1, -1), 4), ("RIGHTPADDING", (0, 0), (-1, -1), 4)]))
    story.append(table)
    story.append(Spacer(1, 5 * mm))

    completeness = report["completeness"]
    limits = [f"Kattavuus: {completeness['state']}"]
    if completeness.get("caller_declared_history_start"):
        limits.append(f"Kutsujan ilmoittama historian saatavuuden alkupäivä (varmistamaton, ei laskennan rajaus): {completeness['caller_declared_history_start']}")
    if completeness.get("caller_declared_limitation"):
        limits.append(f"Ilmoitettu rajaus: {completeness['caller_declared_limitation']}")
    limits.extend(completeness["notes"])
    story.append(Paragraph("<br/>".join(paragraph_text(line) for line in limits), styles["Notice"]))
    if provenance.get("opening_balance_source") is not None:
        story.append(Paragraph("Alkusaldon lähde: " + paragraph_text(json.dumps(provenance["opening_balance_source"], ensure_ascii=False, sort_keys=True)), styles["Small"]))

    def source_description(source: dict) -> str:
        if source["kind"] == "opening_balance":
            return f"Alkusaldo, lähdehash {source['source_head_hash']}"
        detail = f"Tosite #{source['voucher_number']}, alkuperäinen {source['original_posting_date']}, raportissa {source['effective_posting_date']}, hash {source['voucher_hash']}"
        if source.get("attachment_ref"):
            detail += f", lähde {source['attachment_ref']}"
        if source.get("source_fingerprint"):
            detail += f", lähdesormenjälki {source['source_fingerprint']}"
        return detail

    def add_section(section: dict, total_rows: list[tuple[str, object]]) -> None:
        story.append(Spacer(1, 6 * mm))
        story.append(Paragraph(paragraph_text(section["title"]), styles["Heading1"]))
        data = [[Paragraph("Rivi", styles["BodyText"]), Paragraph("Summa", styles["Right"])]]
        for row in section["rows"]:
            data.append([Paragraph(f"{paragraph_text(row['label'])}<br/><font size='7'>{paragraph_text(row['row_id'])}</font>", styles["BodyText"]), Paragraph(f"{fi_amount(row['amount'])} {currency}", styles["Right"])])
        for label, value in total_rows:
            data.append([Paragraph(f"<b>{paragraph_text(label)}</b>", styles["BodyText"]), Paragraph(f"<b>{fi_amount(value)} {currency}</b>", styles["Right"])])
        table = Table(data, colWidths=[132 * mm, 43 * mm], repeatRows=1)
        table.setStyle(TableStyle([("GRID", (0, 0), (-1, -1), .25, colors.HexColor("#ccd4da")), ("BACKGROUND", (0, 0), (-1, 0), colors.HexColor("#e8edef")), ("VALIGN", (0, 0), (-1, -1), "TOP"), ("LEFTPADDING", (0, 0), (-1, -1), 4), ("RIGHTPADDING", (0, 0), (-1, -1), 4)]))
        story.append(table)
        story.append(Paragraph("Tarkastusjälki", styles["Heading2"]))
        traced_rows = [row for row in section["rows"] if row["accounts"]]
        if not traced_rows:
            story.append(Paragraph("Ei tilisaldoja tällä raporttijaksolla.", styles["Small"]))
        for row in traced_rows:
            account_blocks = []
            for account in row["accounts"]:
                account_blocks.append(Paragraph(f"<b>{paragraph_text(account['account'])} {paragraph_text(account['account_name'])}</b> — {fi_amount(account['amount'])} {currency}", styles["Small"]))
                if account["sources"]:
                    for source in account["sources"]:
                        account_blocks.append(Paragraph("• " + paragraph_text(source_description(source)), styles["Small"]))
                else:
                    account_blocks.append(Paragraph("• Ei lähdeviitteitä.", styles["Small"]))
            story.append(KeepTogether([Paragraph(paragraph_text(row["label"]), styles["Heading3"])] + account_blocks))

    month = report["sections"]["month_income"]
    ytd = report["sections"]["year_to_date_income"]
    balance = report["sections"]["balance"]
    add_section(month, [("Kuukauden tulos", month["result"])])
    story.append(PageBreak())
    add_section(ytd, [("Tilikauden tulos", ytd["result"])])
    story.append(PageBreak())
    add_section(balance, [("Vastaavaa yhteensä", balance["assets_total"]), ("Vastattavaa yhteensä", balance["equity_liabilities_total"])])
    story.append(Paragraph("Tase täsmää: " + ("kyllä" if balance["balanced"] else "ei"), styles["BodyText"]))

    story.append(PageBreak())
    story.append(Paragraph("Lähdejoukko", styles["Heading1"]))
    if not report["sources"]:
        story.append(Paragraph("Ei sinetöityjä tositteita raportin rajauksessa.", styles["BodyText"]))
    for source in report["sources"]:
        fields = [
            f"Tosite #{source['voucher_number']}", f"hash {source['voucher_hash']}",
            f"asiakirjapäivä {source['original_document_date']}", f"alkuperäinen kirjauspäivä {source['original_posting_date']}",
            f"raporttipäivä {source['effective_posting_date']}", f"kartta {source['chart_version']}", f"policy {source['policy_set']}",
        ]
        if source.get("attachment_ref"):
            fields.append(f"lähde {source['attachment_ref']}")
        if source.get("source_fingerprint"):
            fields.append(f"lähdesormenjälki {source['source_fingerprint']}")
        story.append(Paragraph("; ".join(paragraph_text(field) for field in fields), styles["Small"]))

    def page(canvas, document):
        canvas.saveState()
        canvas.setFont(font_name, 8)
        canvas.drawString(18 * mm, 12 * mm, f"{report['company']} — {report['period']}")
        canvas.drawRightString(192 * mm, 12 * mm, f"Sivu {document.page}")
        canvas.restoreState()

    try:
        document = SimpleDocTemplate(str(output), pagesize=A4, leftMargin=18 * mm, rightMargin=18 * mm, topMargin=17 * mm, bottomMargin=18 * mm, title=f"{report['company']} – {report['period']}", author="aggountant")
        document.build(story, onFirstPage=page, onLaterPages=page)
    except Exception as error:
        try:
            output.unlink()
        except OSError:
            pass
        fail(f"PDF rendering failed: {error}", 2)


def main() -> None:
    args = parse_args()
    if args.input.resolve() == args.output.resolve():
        fail("input and output paths must differ")
    report = load_report(args.input)
    font = choose_font(args.font)
    render(report, args.output, font)


if __name__ == "__main__":
    main()
