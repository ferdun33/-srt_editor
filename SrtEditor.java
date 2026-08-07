// SrtEditor.java
import java.io.*;
import java.nio.file.*;
import java.time.Duration;
import java.util.*;
import java.util.regex.*;

public class SrtEditor {
    static class Subtitle {
        int index;
        Duration start;
        Duration end;
        String text;
        Subtitle(int idx, Duration s, Duration e, String t) {
            index = idx; start = s; end = e; text = t;
        }
    }

    static Duration parseTime(String s) {
        s = s.replace(',', '.').trim();
        String[] parts = s.split(":");
        if (parts.length != 3) return Duration.ZERO;
        int h = Integer.parseInt(parts[0]);
        int m = Integer.parseInt(parts[1]);
        String[] secPart = parts[2].split("\\.");
        int sec = Integer.parseInt(secPart[0]);
        int ms = secPart.length > 1 ? Integer.parseInt(secPart[1]) : 0;
        return Duration.ofHours(h).plusMinutes(m).plusSeconds(sec).plusMillis(ms);
    }

    static String formatTime(Duration d) {
        long totalSec = d.getSeconds();
        long h = totalSec / 3600;
        long m = (totalSec % 3600) / 60;
        long s = totalSec % 60;
        long ms = d.toMillis() % 1000;
        return String.format("%02d:%02d:%02d,%03d", h, m, s, ms);
    }

    static List<Subtitle> parseSRT(String content) {
        List<Subtitle> subs = new ArrayList<>();
        Pattern block = Pattern.compile("(\\d+)\\s*\\n([\\d:,]+)\\s*-->\\s*([\\d:,]+)\\s*\\n([\\s\\S]*?)(?=\\n\\n|$)",
                                        Pattern.MULTILINE);
        Matcher m = block.matcher(content);
        while (m.find()) {
            int idx = Integer.parseInt(m.group(1));
            Duration start = parseTime(m.group(2));
            Duration end = parseTime(m.group(3));
            String text = m.group(4).trim();
            subs.add(new Subtitle(idx, start, end, text));
        }
        for (int i=0; i<subs.size(); i++) subs.get(i).index = i+1;
        return subs;
    }

    static String formatSRT(List<Subtitle> subs) {
        StringBuilder sb = new StringBuilder();
        for (Subtitle sub : subs) {
            sb.append(sub.index).append("\n");
            sb.append(formatTime(sub.start)).append(" --> ").append(formatTime(sub.end)).append("\n");
            sb.append(sub.text).append("\n\n");
        }
        return sb.toString();
    }

    static class Editor {
        String filename;
        List<Subtitle> subs;

        boolean load(String fname) throws IOException {
            String content = new String(Files.readAllBytes(Paths.get(fname)));
            subs = parseSRT(content);
            filename = fname;
            System.out.println("Загружено " + subs.size() + " субтитров.");
            return true;
        }
        void save(String fname) throws IOException {
            if (fname == null) fname = filename;
            Files.write(Paths.get(fname), formatSRT(subs).getBytes());
            System.out.println("Сохранено.");
        }
        void shift(long ms) {
            Duration delta = Duration.ofMillis(ms);
            for (Subtitle sub : subs) {
                sub.start = sub.start.plus(delta);
                sub.end = sub.end.plus(delta);
            }
            System.out.println("Сдвиг на " + ms + " мс выполнен.");
        }
        void renumber() {
            for (int i=0; i<subs.size(); i++) subs.get(i).index = i+1;
            System.out.println("Перенумерация выполнена.");
        }
        List<String> validate() {
            List<String> errs = new ArrayList<>();
            if (subs.isEmpty()) errs.add("Нет субтитров.");
            Duration prevEnd = Duration.ZERO;
            for (int i=0; i<subs.size(); i++) {
                Subtitle sub = subs.get(i);
                if (sub.start.compareTo(sub.end) >= 0)
                    errs.add("#" + (i+1) + ": начало >= конец");
                if (i>0 && sub.start.compareTo(prevEnd) < 0)
                    errs.add("#" + (i+1) + ": перекрытие с предыдущим");
                prevEnd = sub.end;
            }
            return errs;
        }
        void list() {
            for (Subtitle sub : subs) {
                System.out.printf("#%d  %s --> %s\n%s\n\n", sub.index, formatTime(sub.start), formatTime(sub.end), sub.text);
            }
        }
        void add(String startStr, String endStr, String text) {
            Duration start = parseTime(startStr);
            Duration end = parseTime(endStr);
            subs.add(new Subtitle(subs.size()+1, start, end, text));
            renumber();
            System.out.println("Субтитр добавлен.");
        }
        void edit(int idx, String startStr, String endStr, String text) {
            if (idx<1 || idx>subs.size()) { System.out.println("Неверный номер."); return; }
            Subtitle sub = subs.get(idx-1);
            if (startStr != null && !startStr.isEmpty()) sub.start = parseTime(startStr);
            if (endStr != null && !endStr.isEmpty()) sub.end = parseTime(endStr);
            if (text != null && !text.isEmpty()) sub.text = text;
            System.out.println("Субтитр обновлён.");
        }
        void delete(int idx) {
            if (idx<1 || idx>subs.size()) { System.out.println("Неверный номер."); return; }
            subs.remove(idx-1);
            renumber();
            System.out.println("Субтитр удалён.");
        }
    }

    public static void main(String[] args) throws IOException {
        if (args.length < 1) {
            System.out.println("Использование: java SrtEditor [--shift мс] [--check] [--output file] файл.srt");
            return;
        }
        String filename = null;
        long shiftMs = 0;
        boolean checkOnly = false;
        String outputFile = null;
        for (int i=0; i<args.length; i++) {
            if (args[i].equals("--shift") && i+1 < args.length) {
                shiftMs = Long.parseLong(args[++i]);
            } else if (args[i].equals("--check")) {
                checkOnly = true;
            } else if (args[i].equals("--output") && i+1 < args.length) {
                outputFile = args[++i];
            } else {
                filename = args[i];
            }
        }
        if (filename == null) {
            System.out.println("Не указан файл.");
            return;
        }
        Editor editor = new Editor();
        if (!editor.load(filename)) {
            System.out.println("Ошибка загрузки.");
            return;
        }
        if (checkOnly) {
            List<String> errs = editor.validate();
            if (errs.isEmpty()) System.out.println("Субтитры корректны.");
            else {
                System.out.println("Ошибки:");
                for (String e : errs) System.out.println("  " + e);
            }
            return;
        }
        if (shiftMs != 0) {
            editor.shift(shiftMs);
            editor.save(outputFile != null ? outputFile : filename);
            return;
        }

        Scanner scanner = new Scanner(System.in);
        System.out.println("Редактор субтитров. Введите help для списка команд.");
        while (true) {
            System.out.print("> ");
            String cmd = scanner.nextLine().trim();
            if (cmd.isEmpty()) continue;
            String[] parts = cmd.split("\\s+");
            String op = parts[0];
            switch (op) {
                case "quit": return;
                case "help":
                    System.out.println("list, show <номер>, add, edit <номер> [время_начала] [время_конца] [текст], delete <номер>, shift <мс>, renumber, validate, save, quit");
                    break;
                case "list": editor.list(); break;
                case "show":
                    if (parts.length < 2) { System.out.println("Укажите номер."); break; }
                    int idx = Integer.parseInt(parts[1]);
                    if (idx<1 || idx>editor.subs.size()) { System.out.println("Неверный номер."); break; }
                    Subtitle sub = editor.subs.get(idx-1);
                    System.out.printf("#%d  %s --> %s\n%s\n", sub.index, formatTime(sub.start), formatTime(sub.end), sub.text);
                    break;
                case "add":
                    System.out.print("Начало (HH:MM:SS,mmm): "); String start = scanner.nextLine();
                    System.out.print("Конец (HH:MM:SS,mmm): "); String end = scanner.nextLine();
                    System.out.println("Текст (введите END для завершения):");
                    StringBuilder text = new StringBuilder();
                    while (true) {
                        String line = scanner.nextLine();
                        if (line.equals("END")) break;
                        text.append(line).append("\n");
                    }
                    editor.add(start, end, text.toString());
                    break;
                case "edit":
                    if (parts.length < 2) { System.out.println("Укажите номер."); break; }
                    int idx2 = Integer.parseInt(parts[1]);
                    System.out.print("Новое начало (оставьте пустым): "); String st = scanner.nextLine();
                    System.out.print("Новый конец (оставьте пустым): "); String en = scanner.nextLine();
                    System.out.print("Новый текст (оставьте пустым): "); String tx = scanner.nextLine();
                    editor.edit(idx2, st, en, tx);
                    break;
                case "delete":
                    if (parts.length < 2) { System.out.println("Укажите номер."); break; }
                    int idx3 = Integer.parseInt(parts[1]);
                    editor.delete(idx3);
                    break;
                case "shift":
                    if (parts.length < 2) { System.out.println("Укажите сдвиг в мс."); break; }
                    long ms = Long.parseLong(parts[1]);
                    editor.shift(ms);
                    break;
                case "renumber": editor.renumber(); break;
                case "validate":
                    List<String> errs = editor.validate();
                    if (errs.isEmpty()) System.out.println("OK");
                    else {
                        System.out.println("Ошибки:");
                        for (String e : errs) System.out.println("  " + e);
                    }
                    break;
                case "save":
                    String fname = parts.length > 1 ? parts[1] : null;
                    try { editor.save(fname); } catch (IOException e) { System.out.println("Ошибка сохранения."); }
                    break;
                default: System.out.println("Неизвестная команда.");
            }
        }
    }
}
