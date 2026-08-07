// srt_editor.cs
using System;
using System.Collections.Generic;
using System.IO;
using System.Text.RegularExpressions;

class Subtitle
{
    public int Index { get; set; }
    public TimeSpan Start { get; set; }
    public TimeSpan End { get; set; }
    public string Text { get; set; }
}

class SrtEditor
{
    private string filename;
    private List<Subtitle> subs = new List<Subtitle>();

    public void Load(string fname)
    {
        string content = File.ReadAllText(fname);
        subs = ParseSRT(content);
        filename = fname;
        Console.WriteLine($"Загружено {subs.Count} субтитров.");
    }

    private List<Subtitle> ParseSRT(string content)
    {
        var list = new List<Subtitle>();
        var regex = new Regex(@"(?<idx>\d+)\s*\n(?<start>[\d:,]+)\s*-->\s*(?<end>[\d:,]+)\s*\n(?<text>[\s\S]*?)(?=\n\n|$)",
                               RegexOptions.Multiline);
        foreach (Match m in regex.Matches(content))
        {
            var sub = new Subtitle
            {
                Index = int.Parse(m.Groups["idx"].Value),
                Start = ParseTime(m.Groups["start"].Value),
                End = ParseTime(m.Groups["end"].Value),
                Text = m.Groups["text"].Value.Trim()
            };
            list.Add(sub);
        }
        for (int i=0; i<list.Count; i++) list[i].Index = i+1;
        return list;
    }

    private TimeSpan ParseTime(string s)
    {
        s = s.Replace(',', '.').Trim();
        var parts = s.Split(':');
        var secParts = parts[2].Split('.');
        int h = int.Parse(parts[0]);
        int m = int.Parse(parts[1]);
        int sec = int.Parse(secParts[0]);
        int ms = secParts.Length > 1 ? int.Parse(secParts[1]) : 0;
        return new TimeSpan(0, h, m, sec, ms);
    }

    private string FormatTime(TimeSpan t)
    {
        return $"{t.Hours:D2}:{t.Minutes:D2}:{t.Seconds:D2},{t.Milliseconds:D3}";
    }

    private string FormatSRT()
    {
        var sb = new System.Text.StringBuilder();
        foreach (var sub in subs)
        {
            sb.AppendLine(sub.Index.ToString());
            sb.AppendLine($"{FormatTime(sub.Start)} --> {FormatTime(sub.End)}");
            sb.AppendLine(sub.Text);
            sb.AppendLine();
        }
        return sb.ToString();
    }

    public void Save(string fname = null)
    {
        if (fname == null) fname = filename;
        File.WriteAllText(fname, FormatSRT());
        Console.WriteLine("Сохранено.");
    }

    public void Shift(long ms)
    {
        var delta = TimeSpan.FromMilliseconds(ms);
        foreach (var sub in subs)
        {
            sub.Start = sub.Start.Add(delta);
            sub.End = sub.End.Add(delta);
        }
        Console.WriteLine($"Сдвиг на {ms} мс выполнен.");
    }

    public void Renumber()
    {
        for (int i=0; i<subs.Count; i++) subs[i].Index = i+1;
        Console.WriteLine("Перенумерация выполнена.");
    }

    public List<string> Validate()
    {
        var errs = new List<string>();
        if (subs.Count == 0) errs.Add("Нет субтитров.");
        TimeSpan prevEnd = TimeSpan.Zero;
        for (int i=0; i<subs.Count; i++)
        {
            if (subs[i].Start >= subs[i].End)
                errs.Add($"#{i+1}: начало >= конец");
            if (i>0 && subs[i].Start < prevEnd)
                errs.Add($"#{i+1}: перекрытие с предыдущим");
            prevEnd = subs[i].End;
        }
        return errs;
    }

    public void List()
    {
        foreach (var sub in subs)
            Console.WriteLine($"#{sub.Index}  {FormatTime(sub.Start)} --> {FormatTime(sub.End)}\n{sub.Text}\n");
    }

    public void Add(string startStr, string endStr, string text)
    {
        var start = ParseTime(startStr);
        var end = ParseTime(endStr);
        subs.Add(new Subtitle { Index = subs.Count+1, Start = start, End = end, Text = text });
        Renumber();
        Console.WriteLine("Субтитр добавлен.");
    }

    public void Edit(int idx, string startStr, string endStr, string text)
    {
        if (idx<1 || idx>subs.Count) { Console.WriteLine("Неверный номер."); return; }
        var sub = subs[idx-1];
        if (!string.IsNullOrEmpty(startStr)) sub.Start = ParseTime(startStr);
        if (!string.IsNullOrEmpty(endStr)) sub.End = ParseTime(endStr);
        if (!string.IsNullOrEmpty(text)) sub.Text = text;
        Console.WriteLine("Субтитр обновлён.");
    }

    public void Delete(int idx)
    {
        if (idx<1 || idx>subs.Count) { Console.WriteLine("Неверный номер."); return; }
        subs.RemoveAt(idx-1);
        Renumber();
        Console.WriteLine("Субтитр удалён.");
    }

    public static void Main(string[] args)
    {
        if (args.Length < 1)
        {
            Console.WriteLine("Использование: dotnet run [--shift мс] [--check] [--output file] файл.srt");
            return;
        }
        string filename = null;
        long shiftMs = 0;
        bool checkOnly = false;
        string outputFile = null;
        for (int i=0; i<args.Length; i++)
        {
            if (args[i] == "--shift" && i+1 < args.Length)
            {
                shiftMs = long.Parse(args[++i]);
            }
            else if (args[i] == "--check")
            {
                checkOnly = true;
            }
            else if (args[i] == "--output" && i+1 < args.Length)
            {
                outputFile = args[++i];
            }
            else
            {
                filename = args[i];
            }
        }
        if (filename == null)
        {
            Console.WriteLine("Не указан файл.");
            return;
        }
        var editor = new SrtEditor();
        try
        {
            editor.Load(filename);
        }
        catch
        {
            Console.WriteLine("Ошибка загрузки.");
            return;
        }
        if (checkOnly)
        {
            var errs = editor.Validate();
            if (errs.Count == 0) Console.WriteLine("Субтитры корректны.");
            else
            {
                Console.WriteLine("Ошибки:");
                foreach (var e in errs) Console.WriteLine("  " + e);
            }
            return;
        }
        if (shiftMs != 0)
        {
            editor.Shift(shiftMs);
            editor.Save(outputFile ?? filename);
            return;
        }

        Console.WriteLine("Редактор субтитров. Введите help для списка команд.");
        while (true)
        {
            Console.Write("> ");
            string cmd = Console.ReadLine().Trim();
            if (string.IsNullOrEmpty(cmd)) continue;
            string[] parts = cmd.Split(' ');
            string op = parts[0];
            switch (op)
            {
                case "quit": return;
                case "help":
                    Console.WriteLine("list, show <номер>, add, edit <номер> [время_начала] [время_конца] [текст], delete <номер>, shift <мс>, renumber, validate, save, quit");
                    break;
                case "list": editor.List(); break;
                case "show":
                    if (parts.Length < 2) { Console.WriteLine("Укажите номер."); break; }
                    int idx = int.Parse(parts[1]);
                    if (idx<1 || idx>editor.subs.Count) { Console.WriteLine("Неверный номер."); break; }
                    var sub = editor.subs[idx-1];
                    Console.WriteLine($"#{sub.Index}  {editor.FormatTime(sub.Start)} --> {editor.FormatTime(sub.End)}\n{sub.Text}");
                    break;
                case "add":
                    Console.Write("Начало (HH:MM:SS,mmm): "); string start = Console.ReadLine();
                    Console.Write("Конец (HH:MM:SS,mmm): "); string end = Console.ReadLine();
                    Console.WriteLine("Текст (введите END для завершения):");
                    var text = new System.Text.StringBuilder();
                    while (true)
                    {
                        string line = Console.ReadLine();
                        if (line == "END") break;
                        text.AppendLine(line);
                    }
                    editor.Add(start, end, text.ToString());
                    break;
                case "edit":
                    if (parts.Length < 2) { Console.WriteLine("Укажите номер."); break; }
                    int idx2 = int.Parse(parts[1]);
                    Console.Write("Новое начало (оставьте пустым): "); string st = Console.ReadLine();
                    Console.Write("Новый конец (оставьте пустым): "); string en = Console.ReadLine();
                    Console.Write("Новый текст (оставьте пустым): "); string tx = Console.ReadLine();
                    editor.Edit(idx2, st, en, tx);
                    break;
                case "delete":
                    if (parts.Length < 2) { Console.WriteLine("Укажите номер."); break; }
                    int idx3 = int.Parse(parts[1]);
                    editor.Delete(idx3);
                    break;
                case "shift":
                    if (parts.Length < 2) { Console.WriteLine("Укажите сдвиг в мс."); break; }
                    long ms = long.Parse(parts[1]);
                    editor.Shift(ms);
                    break;
                case "renumber": editor.Renumber(); break;
                case "validate":
                    var errs = editor.Validate();
                    if (errs.Count == 0) Console.WriteLine("OK");
                    else
                    {
                        Console.WriteLine("Ошибки:");
                        foreach (var e in errs) Console.WriteLine("  " + e);
                    }
                    break;
                case "save":
                    string fname = parts.Length > 1 ? parts[1] : null;
                    editor.Save(fname);
                    break;
                default: Console.WriteLine("Неизвестная команда."); break;
            }
        }
    }
}
