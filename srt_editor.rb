# srt_editor.rb
class Subtitle
  attr_accessor :index, :start, :end, :text
  def initialize(index, start, end_, text)
    @index = index
    @start = start  # секунды с миллисекундами (Float)
    @end = end_
    @text = text
  end
end

def parse_time(s)
  s = s.strip.tr(',', '.')
  h, m, sec = s.split(':')
  sec, ms = sec.split('.')
  h.to_i * 3600 + m.to_i * 60 + sec.to_i + (ms || 0).to_f / 1000
end

def format_time(t)
  h = t.to_i / 3600
  m = (t.to_i % 3600) / 60
  s = t.to_i % 60
  ms = ((t - t.to_i) * 1000).round
  format("%02d:%02d:%02d,%03d", h, m, s, ms)
end

def parse_srt(content)
  subs = []
  blocks = content.strip.split(/\n\s*\n/)
  blocks.each do |block|
    lines = block.lines.map(&:chomp)
    next if lines.size < 3
    idx = lines[0].to_i
    time_line = lines[1]
    from, to = time_line.split('-->').map(&:strip)
    start = parse_time(from)
    end_ = parse_time(to)
    text = lines[2..-1].join("\n")
    subs << Subtitle.new(idx, start, end_, text)
  end
  subs.each_with_index { |s, i| s.index = i+1 }
  subs
end

def format_srt(subs)
  subs.map do |s|
    "#{s.index}\n#{format_time(s.start)} --> #{format_time(s.end)}\n#{s.text}"
  end.join("\n\n") + "\n"
end

class Editor
  attr_reader :filename, :subs

  def initialize(filename)
    @filename = filename
    @subs = parse_srt(File.read(filename, encoding: 'utf-8'))
    puts "Загружено #{@subs.size} субтитров."
  end

  def save(fname = nil)
    fname ||= @filename
    File.write(fname, format_srt(@subs), encoding: 'utf-8')
    puts "Сохранено."
  end

  def shift(ms)
    delta = ms / 1000.0
    @subs.each do |s|
      s.start += delta
      s.end += delta
    end
    puts "Сдвиг на #{ms} мс выполнен."
  end

  def renumber
    @subs.each_with_index { |s, i| s.index = i+1 }
    puts "Перенумерация выполнена."
  end

  def validate
    errs = []
    if @subs.empty?
      errs << "Нет субтитров."
    else
      prev_end = 0
      @subs.each_with_index do |s, i|
        if s.start >= s.end
          errs << "##{i+1}: начало >= конец"
        end
        if i > 0 && s.start < prev_end
          errs << "##{i+1}: перекрытие с предыдущим"
        end
        prev_end = s.end
      end
    end
    errs
  end

  def list
    @subs.each do |s|
      puts "#{s.index}  #{format_time(s.start)} --> #{format_time(s.end)}\n#{s.text}\n"
    end
  end

  def add(start_str, end_str, text)
    start = parse_time(start_str)
    end_ = parse_time(end_str)
    @subs << Subtitle.new(@subs.size+1, start, end_, text)
    renumber
    puts "Субтитр добавлен."
  end

  def edit(idx, start_str, end_str, text)
    if idx < 1 || idx > @subs.size
      puts "Неверный номер."
      return
    end
    s = @subs[idx-1]
    s.start = parse_time(start_str) if start_str && !start_str.empty?
    s.end = parse_time(end_str) if end_str && !end_str.empty?
    s.text = text if text && !text.empty?
    puts "Субтитр обновлён."
  end

  def delete(idx)
    if idx < 1 || idx > @subs.size
      puts "Неверный номер."
      return
    end
    @subs.delete_at(idx-1)
    renumber
    puts "Субтитр удалён."
  end
end

if __FILE__ == $0
  if ARGV.empty?
    puts "Использование: ruby srt_editor.rb [--shift мс] [--check] [--output file] файл.srt"
    exit
  end
  filename = nil
  shift_ms = 0
  check_only = false
  output_file = nil
  i = 0
  while i < ARGV.size
    case ARGV[i]
    when '--shift'
      shift_ms = ARGV[i+1].to_i
      i += 2
    when '--check'
      check_only = true
      i += 1
    when '--output'
      output_file = ARGV[i+1]
      i += 2
    else
      filename = ARGV[i]
      i += 1
    end
  end
  unless filename
    puts "Не указан файл."
    exit
  end
  begin
    editor = Editor.new(filename)
  rescue => e
    puts "Ошибка загрузки: #{e.message}"
    exit
  end
  if check_only
    errs = editor.validate
    if errs.empty?
      puts "Субтитры корректны."
    else
      puts "Ошибки:"
      errs.each { |e| puts "  #{e}" }
    end
    exit
  end
  if shift_ms != 0
    editor.shift(shift_ms)
    editor.save(output_file || filename)
    exit
  end

  puts "Редактор субтитров. Введите help для списка команд."
  loop do
    print "> "
    cmd = gets.chomp.strip
    next if cmd.empty?
    parts = cmd.split
    op = parts[0]
    case op
    when 'quit'
      break
    when 'help'
      puts "list, show <номер>, add, edit <номер> [время_начала] [время_конца] [текст], delete <номер>, shift <мс>, renumber, validate, save, quit"
    when 'list'
      editor.list
    when 'show'
      if parts.size < 2
        puts "Укажите номер."
        next
      end
      idx = parts[1].to_i
      if idx < 1 || idx > editor.subs.size
        puts "Неверный номер."
        next
      end
      s = editor.subs[idx-1]
      puts "#{s.index}  #{format_time(s.start)} --> #{format_time(s.end)}\n#{s.text}"
    when 'add'
      print "Начало (HH:MM:SS,mmm): "
      start = gets.chomp
      print "Конец (HH:MM:SS,mmm): "
      end_ = gets.chomp
      puts "Текст (введите END для завершения):"
      lines = []
      while (line = gets.chomp) != 'END'
        lines << line
      end
      editor.add(start, end_, lines.join("\n"))
    when 'edit'
      if parts.size < 2
        puts "Укажите номер."
        next
      end
      idx = parts[1].to_i
      print "Новое начало (оставьте пустым): "
      start = gets.chomp
      print "Новый конец (оставьте пустым): "
      end_ = gets.chomp
      print "Новый текст (оставьте пустым): "
      text = gets.chomp
      editor.edit(idx, start, end_, text)
    when 'delete'
      if parts.size < 2
        puts "Укажите номер."
        next
      end
      idx = parts[1].to_i
      editor.delete(idx)
    when 'shift'
      if parts.size < 2
        puts "Укажите сдвиг в мс."
        next
      end
      ms = parts[1].to_i
      editor.shift(ms)
    when 'renumber'
      editor.renumber
    when 'validate'
      errs = editor.validate
      if errs.empty?
        puts "OK"
      else
        puts "Ошибки:"
        errs.each { |e| puts "  #{e}" }
      end
    when 'save'
      fname = parts[1] if parts.size > 1
      editor.save(fname)
    else
      puts "Неизвестная команда."
    end
  end
end
