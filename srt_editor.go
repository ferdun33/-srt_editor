// srt_editor.go
package main

import (
	"bufio"
	"fmt"
	"os"
	"strconv"
	"strings"
	"time"
)

type Subtitle struct {
	Index int
	Start time.Duration
	End   time.Duration
	Text  string
}

func parseTime(s string) (time.Duration, error) {
	s = strings.TrimSpace(s)
	s = strings.Replace(s, ",", ".", 1)
	parts := strings.Split(s, ":")
	if len(parts) != 3 {
		return 0, fmt.Errorf("invalid time format")
	}
	h, _ := strconv.Atoi(parts[0])
	m, _ := strconv.Atoi(parts[1])
	secParts := strings.Split(parts[2], ".")
	sec, _ := strconv.Atoi(secParts[0])
	ms := 0
	if len(secParts) > 1 {
		ms, _ = strconv.Atoi(secParts[1])
	}
	d := time.Duration(h)*time.Hour + time.Duration(m)*time.Minute + time.Duration(sec)*time.Second + time.Duration(ms)*time.Millisecond
	return d, nil
}

func formatTime(d time.Duration) string {
	h := int(d.Hours())
	m := int(d.Minutes()) % 60
	s := int(d.Seconds()) % 60
	ms := int(d.Milliseconds()) % 1000
	return fmt.Sprintf("%02d:%02d:%02d,%03d", h, m, s, ms)
}

func parseSRT(content string) []Subtitle {
	var subs []Subtitle
	blocks := strings.Split(strings.TrimSpace(content), "\n\n")
	for _, block := range blocks {
		lines := strings.Split(block, "\n")
		if len(lines) < 3 {
			continue
		}
		var idx int
		var start time.Duration
		var end time.Duration
		var textLines []string
		// первый может быть номером
		lineIdx := 0
		if idx, err := strconv.Atoi(lines[0]); err == nil {
			lineIdx = 1
			_ = idx // мы будем перенумеровывать
		}
		if lineIdx >= len(lines) {
			continue
		}
		timeLine := lines[lineIdx]
		parts := strings.Split(timeLine, "-->")
		if len(parts) != 2 {
			continue
		}
		start, err1 := parseTime(parts[0])
		end, err2 := parseTime(parts[1])
		if err1 != nil || err2 != nil {
			continue
		}
		text := strings.Join(lines[lineIdx+1:], "\n")
		sub := Subtitle{Index: 0, Start: start, End: end, Text: text}
		subs = append(subs, sub)
	}
	// перенумеровываем
	for i := range subs {
		subs[i].Index = i + 1
	}
	return subs
}

func formatSRT(subs []Subtitle) string {
	var lines []string
	for _, sub := range subs {
		lines = append(lines, strconv.Itoa(sub.Index))
		lines = append(lines, fmt.Sprintf("%s --> %s", formatTime(sub.Start), formatTime(sub.End)))
		lines = append(lines, sub.Text)
		lines = append(lines, "")
	}
	return strings.Join(lines, "\n")
}

type Editor struct {
	filename string
	subs     []Subtitle
}

func (e *Editor) load(filename string) error {
	data, err := os.ReadFile(filename)
	if err != nil {
		return err
	}
	e.subs = parseSRT(string(data))
	e.filename = filename
	fmt.Printf("Загружено %d субтитров.\n", len(e.subs))
	return nil
}

func (e *Editor) save(filename string) error {
	if filename == "" {
		filename = e.filename
	}
	content := formatSRT(e.subs)
	return os.WriteFile(filename, []byte(content), 0644)
}

func (e *Editor) shift(ms int) {
	delta := time.Duration(ms) * time.Millisecond
	for i := range e.subs {
		e.subs[i].Start += delta
		e.subs[i].End += delta
	}
	fmt.Printf("Сдвиг на %d мс выполнен.\n", ms)
}

func (e *Editor) renumber() {
	for i := range e.subs {
		e.subs[i].Index = i + 1
	}
	fmt.Println("Перенумерация выполнена.")
}

func (e *Editor) validate() []string {
	var errs []string
	if len(e.subs) == 0 {
		errs = append(errs, "Нет субтитров.")
	}
	var prevEnd time.Duration
	for i, sub := range e.subs {
		if sub.Start >= sub.End {
			errs = append(errs, fmt.Sprintf("#%d: начало >= конец", i+1))
		}
		if i > 0 && sub.Start < prevEnd {
			errs = append(errs, fmt.Sprintf("#%d: перекрытие с предыдущим", i+1))
		}
		prevEnd = sub.End
	}
	return errs
}

func (e *Editor) list() {
	for _, sub := range e.subs {
		fmt.Printf("#%d  %s --> %s\n%s\n\n", sub.Index, formatTime(sub.Start), formatTime(sub.End), sub.Text)
	}
}

func (e *Editor) add(startStr, endStr, text string) {
	start, err1 := parseTime(startStr)
	end, err2 := parseTime(endStr)
	if err1 != nil || err2 != nil {
		fmt.Println("Неверный формат времени.")
		return
	}
	sub := Subtitle{Index: len(e.subs) + 1, Start: start, End: end, Text: text}
	e.subs = append(e.subs, sub)
	e.renumber()
	fmt.Println("Субтитр добавлен.")
}

func (e *Editor) edit(idx int, startStr, endStr, text string) {
	if idx < 1 || idx > len(e.subs) {
		fmt.Println("Неверный номер.")
		return
	}
	sub := &e.subs[idx-1]
	if startStr != "" {
		if st, err := parseTime(startStr); err == nil {
			sub.Start = st
		}
	}
	if endStr != "" {
		if en, err := parseTime(endStr); err == nil {
			sub.End = en
		}
	}
	if text != "" {
		sub.Text = text
	}
	fmt.Println("Субтитр обновлён.")
}

func (e *Editor) delete(idx int) {
	if idx < 1 || idx > len(e.subs) {
		fmt.Println("Неверный номер.")
		return
	}
	e.subs = append(e.subs[:idx-1], e.subs[idx:]...)
	e.renumber()
	fmt.Println("Субтитр удалён.")
}

func main() {
	if len(os.Args) < 2 {
		fmt.Println("Использование: go run srt_editor.go [--shift мс] [--check] [--output file] файл.srt")
		return
	}
	args := os.Args[1:]
	filename := ""
	shiftMs := 0
	checkOnly := false
	outputFile := ""
	for i := 0; i < len(args); i++ {
		switch args[i] {
		case "--shift":
			if i+1 < len(args) {
				shiftMs, _ = strconv.Atoi(args[i+1])
				i++
			}
		case "--check":
			checkOnly = true
		case "--output":
			if i+1 < len(args) {
				outputFile = args[i+1]
				i++
			}
		default:
			filename = args[i]
		}
	}
	if filename == "" {
		fmt.Println("Не указан файл.")
		return
	}
	editor := &Editor{}
	if err := editor.load(filename); err != nil {
		fmt.Println("Ошибка загрузки:", err)
		return
	}
	if checkOnly {
		errs := editor.validate()
		if len(errs) == 0 {
			fmt.Println("Субтитры корректны.")
		} else {
			fmt.Println("Ошибки:")
			for _, e := range errs {
				fmt.Println("  " + e)
			}
		}
		return
	}
	if shiftMs != 0 {
		editor.shift(shiftMs)
		if outputFile != "" {
			editor.save(outputFile)
		} else {
			editor.save("")
		}
		return
	}
	// interactive
	scanner := bufio.NewScanner(os.Stdin)
	fmt.Println("Редактор субтитров. Введите help для списка команд.")
	for {
		fmt.Print("> ")
		if !scanner.Scan() {
			break
		}
		cmd := strings.TrimSpace(scanner.Text())
		if cmd == "" {
			continue
		}
		parts := strings.Fields(cmd)
		op := parts[0]
		switch op {
		case "quit":
			return
		case "help":
			fmt.Println("list, show <номер>, add, edit <номер> [время_начала] [время_конца] [текст], delete <номер>, shift <мс>, renumber, validate, save, quit")
		case "list":
			editor.list()
		case "show":
			if len(parts) < 2 {
				fmt.Println("Укажите номер.")
				continue
			}
			idx, _ := strconv.Atoi(parts[1])
			if idx < 1 || idx > len(editor.subs) {
				fmt.Println("Неверный номер.")
			} else {
				sub := editor.subs[idx-1]
				fmt.Printf("#%d  %s --> %s\n%s\n", sub.Index, formatTime(sub.Start), formatTime(sub.End), sub.Text)
			}
		case "add":
			fmt.Print("Начало (HH:MM:SS,mmm): ")
			scanner.Scan()
			start := scanner.Text()
			fmt.Print("Конец (HH:MM:SS,mmm): ")
			scanner.Scan()
			end := scanner.Text()
			fmt.Print("Текст (введите END для завершения): ")
			var lines []string
			for {
				scanner.Scan()
				line := scanner.Text()
				if line == "END" {
					break
				}
				lines = append(lines, line)
			}
			text := strings.Join(lines, "\n")
			editor.add(start, end, text)
		case "edit":
			if len(parts) < 2 {
				fmt.Println("Укажите номер.")
				continue
			}
			idx, _ := strconv.Atoi(parts[1])
			fmt.Print("Новое начало (оставьте пустым): ")
			scanner.Scan()
			start := scanner.Text()
			fmt.Print("Новый конец (оставьте пустым): ")
			scanner.Scan()
			end := scanner.Text()
			fmt.Print("Новый текст (оставьте пустым): ")
			scanner.Scan()
			text := scanner.Text()
			editor.edit(idx, start, end, text)
		case "delete":
			if len(parts) < 2 {
				fmt.Println("Укажите номер.")
				continue
			}
			idx, _ := strconv.Atoi(parts[1])
			editor.delete(idx)
		case "shift":
			if len(parts) < 2 {
				fmt.Println("Укажите сдвиг в мс.")
				continue
			}
			ms, _ := strconv.Atoi(parts[1])
			editor.shift(ms)
		case "renumber":
			editor.renumber()
		case "validate":
			errs := editor.validate()
			if len(errs) == 0 {
				fmt.Println("OK")
			} else {
				fmt.Println("Ошибки:")
				for _, e := range errs {
					fmt.Println("  " + e)
				}
			}
		case "save":
			if len(parts) > 1 {
				editor.save(parts[1])
			} else {
				editor.save("")
			}
			fmt.Println("Сохранено.")
		default:
			fmt.Println("Неизвестная команда.")
		}
	}
}
