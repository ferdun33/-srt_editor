# srt_editor.py
import sys
import re
import datetime
from collections import OrderedDict

class SrtSubtitle:
    def __init__(self, index, start, end, text):
        self.index = index
        self.start = start  # timedelta
        self.end = end      # timedelta
        self.text = text

    def __repr__(self):
        return f"#{self.index} {self.start} --> {self.end}\n{self.text}"

def parse_time(t_str):
    """Парсит время вида 00:00:00,000 или 00:00:00.000"""
    t_str = t_str.strip()
    # замена запятой на точку для унификации
    t_str = t_str.replace(',', '.')
    try:
        parts = t_str.split(':')
        if len(parts) != 3:
            return None
        h = int(parts[0])
        m = int(parts[1])
        sec_part = parts[2].split('.')
        s = int(sec_part[0])
        ms = int(sec_part[1]) if len(sec_part) > 1 else 0
        return datetime.timedelta(hours=h, minutes=m, seconds=s, milliseconds=ms)
    except:
        return None

def format_time(td):
    """Форматирует timedelta в SRT-формат HH:MM:SS,mmm"""
    total_sec = int(td.total_seconds())
    ms = int(td.microseconds / 1000)
    h = total_sec // 3600
    m = (total_sec % 3600) // 60
    s = total_sec % 60
    return f"{h:02d}:{m:02d}:{s:02d},{ms:03d}"

def parse_srt(content):
    """Парсит SRT-файл, возвращает список Subtitle"""
    subtitles = []
    blocks = re.split(r'\n\s*\n', content.strip())
    for block in blocks:
        lines = block.strip().splitlines()
        if not lines:
            continue
        # первая строка - номер (иногда отсутствует)
        idx = 0
        if lines[0].isdigit():
            idx = int(lines[0])
            lines = lines[1:]
        if len(lines) < 2:
            continue
        # следующая строка - тайминг
        time_line = lines[0]
        parts = time_line.split('-->')
        if len(parts) != 2:
            continue
        start = parse_time(parts[0].strip())
        end = parse_time(parts[1].strip())
        if start is None or end is None:
            continue
        text = '\n'.join(lines[1:]).strip()
        sub = SrtSubtitle(idx, start, end, text)
        subtitles.append(sub)
    return subtitles

def format_srt(subtitles):
    """Форматирует список субтитров в SRT-строку"""
    out = []
    for i, sub in enumerate(subtitles, 1):
        sub.index = i  # перенумеровываем
        out.append(str(i))
        out.append(f"{format_time(sub.start)} --> {format_time(sub.end)}")
        out.append(sub.text)
        out.append('')
    return '\n'.join(out)

class SrtEditor:
    def __init__(self, filename=None):
        self.filename = filename
        self.subtitles = []
        if filename:
            self.load(filename)

    def load(self, filename):
        try:
            with open(filename, 'r', encoding='utf-8') as f:
                content = f.read()
            self.subtitles = parse_srt(content)
            self.filename = filename
            print(f"Загружено {len(self.subtitles)} субтитров.")
        except Exception as e:
            print(f"Ошибка загрузки: {e}")
            self.subtitles = []

    def save(self, filename=None):
        if not filename:
            filename = self.filename
        if not filename:
            print("Не указано имя файла.")
            return
        content = format_srt(self.subtitles)
        with open(filename, 'w', encoding='utf-8') as f:
            f.write(content)
        print(f"Сохранено в {filename}")

    def shift(self, ms):
        """Сдвигает все субтитры на ms миллисекунд (+/-)"""
        delta = datetime.timedelta(milliseconds=ms)
        for sub in self.subtitles:
            sub.start += delta
            sub.end += delta
        print(f"Сдвиг на {ms} мс выполнен.")

    def renumber(self):
        for i, sub in enumerate(self.subtitles, 1):
            sub.index = i
        print("Перенумерация выполнена.")

    def validate(self):
        errors = []
        if not self.subtitles:
            errors.append("Нет субтитров.")
        prev_end = None
        for i, sub in enumerate(self.subtitles):
            if sub.start >= sub.end:
                errors.append(f"#{i+1}: начало >= конец")
            if prev_end and sub.start < prev_end:
                errors.append(f"#{i+1}: перекрытие с предыдущим (пред. конец {prev_end})")
            prev_end = sub.end
        return errors

    def list(self):
        for sub in self.subtitles:
            print(sub)

    def add(self, start_str, end_str, text):
        start = parse_time(start_str)
        end = parse_time(end_str)
        if start is None or end is None:
            print("Неверный формат времени.")
            return
        sub = SrtSubtitle(len(self.subtitles)+1, start, end, text)
        self.subtitles.append(sub)
        self.renumber()
        print("Субтитр добавлен.")

    def edit(self, idx, start_str=None, end_str=None, text=None):
        if idx < 1 or idx > len(self.subtitles):
            print("Неверный номер.")
            return
        sub = self.subtitles[idx-1]
        if start_str:
            st = parse_time(start_str)
            if st: sub.start = st
        if end_str:
            en = parse_time(end_str)
            if en: sub.end = en
        if text:
            sub.text = text
        print("Субтитр обновлён.")

    def delete(self, idx):
        if idx < 1 or idx > len(self.subtitles):
            print("Неверный номер.")
            return
        del self.subtitles[idx-1]
        self.renumber()
        print("Субтитр удалён.")

def main():
    if len(sys.argv) < 2:
        print("Использование: python srt_editor.py [--shift мс] [--check] [--output file] файл.srt")
        return
    args = sys.argv[1:]
    filename = None
    shift_ms = None
    check_only = False
    output_file = None
    i = 0
    while i < len(args):
        if args[i] == '--shift' and i+1 < len(args):
            shift_ms = int(args[i+1])
            i += 2
        elif args[i] == '--check':
            check_only = True
            i += 1
        elif args[i] == '--output' and i+1 < len(args):
            output_file = args[i+1]
            i += 2
        else:
            filename = args[i]
            i += 1

    if not filename:
        print("Не указан файл.")
        return

    editor = SrtEditor(filename)
    if check_only:
        errors = editor.validate()
        if errors:
            print("Обнаружены ошибки:")
            for e in errors:
                print("  " + e)
        else:
            print("Субтитры корректны.")
        return

    if shift_ms is not None:
        editor.shift(shift_ms)
        if output_file:
            editor.save(output_file)
        else:
            editor.save()
        return

    # Интерактивный режим
    print("Редактор субтитров. Введите help для списка команд.")
    while True:
        cmd = input("> ").strip()
        if not cmd:
            continue
        parts = cmd.split()
        op = parts[0].lower()
        if op == 'quit':
            break
        elif op == 'help':
            print("list, show <номер>, add, edit <номер> [время_начала] [время_конца] [текст], delete <номер>, shift <мс>, renumber, validate, save, quit")
        elif op == 'list':
            editor.list()
        elif op == 'show':
            if len(parts) < 2:
                print("Укажите номер.")
                continue
            try:
                idx = int(parts[1])
                if 1 <= idx <= len(editor.subtitles):
                    print(editor.subtitles[idx-1])
                else:
                    print("Неверный номер.")
            except:
                print("Неверный номер.")
        elif op == 'add':
            # запрос параметров
            start = input("Начало (HH:MM:SS,mmm): ")
            end = input("Конец (HH:MM:SS,mmm): ")
            text = input("Текст (Enter для завершения, введите 'END' для окончания): ")
            lines = []
            while True:
                line = input()
                if line == 'END':
                    break
                lines.append(line)
            text = '\n'.join(lines)
            editor.add(start, end, text)
        elif op == 'edit':
            if len(parts) < 2:
                print("Укажите номер.")
                continue
            idx = int(parts[1])
            start = input("Новое начало (оставьте пустым для пропуска): ")
            end = input("Новый конец (оставьте пустым для пропуска): ")
            text = input("Новый текст (оставьте пустым для пропуска): ")
            editor.edit(idx, start or None, end or None, text or None)
        elif op == 'delete':
            if len(parts) < 2:
                print("Укажите номер.")
                continue
            idx = int(parts[1])
            editor.delete(idx)
        elif op == 'shift':
            if len(parts) < 2:
                print("Укажите сдвиг в мс.")
                continue
            try:
                ms = int(parts[1])
                editor.shift(ms)
            except:
                print("Неверное значение.")
        elif op == 'renumber':
            editor.renumber()
        elif op == 'validate':
            errors = editor.validate()
            if errors:
                print("Ошибки:")
                for e in errors:
                    print("  " + e)
            else:
                print("OK")
        elif op == 'save':
            if len(parts) > 1:
                editor.save(parts[1])
            else:
                editor.save()
        else:
            print("Неизвестная команда.")

if __name__ == '__main__':
    main()
