package main

import (
	"bytes"
	"encoding/binary"
	"strings"
	"net"
	"fmt"
	"os"

	"inoli-tui/ipc"
	"inoli-tui/ipc/message"

	"github.com/charmbracelet/bubbles/help"
	"github.com/charmbracelet/bubbles/key"
	"github.com/charmbracelet/lipgloss"
	tea "github.com/charmbracelet/bubbletea"
)

type TitleBox struct {
	BoxStyle   lipgloss.Style
	TitleStyle lipgloss.Style
}

type version struct {
	major       int
	minor       int
	maintenance int
	build       int
}

type keymap struct {
	quit    key.Binding
	refresh key.Binding
}

type batteryMsg   struct { value byte   }
type heartrateMsg struct { value byte   }
type stepsMsg     struct { value uint32 }

type model struct {
	title     string
	battery   byte
	heartrate byte
	steps     uint32
	firmware  version

	help      help.Model
	keymap    keymap
}

func (m model) Init() tea.Cmd {
	return nil
}

func (m model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case batteryMsg:
		m.battery = msg.value
	case heartrateMsg:
		m.heartrate = msg.value
	case stepsMsg:
		m.steps = msg.value
	case tea.KeyMsg:
		switch {
		case key.Matches(msg, m.keymap.refresh):
			return m, nil
		case key.Matches(msg, m.keymap.quit):
			println("\n")
			return m, tea.Quit
		}
	}

	return m, nil
}

func (m model) UpdateBattery(level byte) {
	
}

var (
	style = lipgloss.NewStyle().Border(lipgloss.RoundedBorder()).Padding(0, 10, 0, 2)

	battery = lipgloss.NewStyle().SetString(" ").String()
	heart   = lipgloss.NewStyle().SetString("♥ ").Foreground(lipgloss.Color("#FF4D94")).String()
	human   = lipgloss.NewStyle().SetString(" ").String()
)

func (m model) helpView() string {
	return "\n" + m.help.ShortHelpView([]key.Binding{
		m.keymap.quit, m.keymap.refresh,
	})
}

func (m model) View() string {
	var s = strings.Builder{}

	var battery = fmt.Sprintf("%s: %d%%\n", battery, m.battery)
	var heartrate = fmt.Sprintf("%s: %d BPM\n", heart, m.heartrate)
	var steps = fmt.Sprintf("%s: %d", human, m.steps)

	//s.WriteString(title)
	s.WriteString(battery)
	s.WriteString(heartrate)
	s.WriteString(steps)
	//s.WriteString(firmware)

	return style.Render(s.String()) + m.helpView()
}

func connect(socket string) (net.Conn, error) {
	var conn, err = net.Dial("unix", socket)
	if err != nil { return nil, err }

	ipc.RequestName(conn)
	if err != nil { return nil, err }

	ipc.RequestBattery(conn)
	if err != nil { return nil, err }

	ipc.RequestHeartrateContinuous(conn)
	if err != nil { return nil, err }

	ipc.RequestSteps(conn)
	if err != nil { return nil, err }

	return conn, nil
}

func handleMessages(conn net.Conn, p *tea.Program) error {
	var buf = make([]byte, 16)
	var magic = []byte { 'M', 'S', 'G' }

	for {
		var _, err = conn.Read(buf)
		if err != nil { return err }

		if !bytes.Equal(buf[0:len(magic)], magic) {
			continue
		}

		var r = bytes.NewReader(buf[len(magic):])

		_kind, err := r.ReadByte()
		if err != nil { panic("couldn't read message payload - EOF") }

		var kind = message.Message(_kind)

		switch kind {
		case message.Battery:
			value, err := r.ReadByte()
			if err != nil { panic("couldn't read message payload - EOF") }
			p.Send(batteryMsg { value })
		case message.Heartrate:
			value, err := r.ReadByte()
			if err != nil { panic("couldn't read message payload - EOF") }
			p.Send(heartrateMsg { value })
		case message.Steps:
			var value uint32
			binary.Read(r, binary.LittleEndian, &value)
			p.Send(stepsMsg { value })
		}
	}
}

func main() {
	var conn, err = connect("../socket")

	model := model {
		title:     "",
		battery:   0,
		heartrate: 0,
		steps:     0,

		help:   help.NewModel(),
		keymap: keymap {
			quit: key.NewBinding (
				key.WithKeys("ctrl+c", "q"),
				key.WithHelp("q:", "Quit"),
			),
			refresh: key.NewBinding (
				key.WithKeys("r"),
				key.WithHelp("r:", "Refresh"),
			),
		},
	}

	p := tea.NewProgram(model)

	if err == nil {

		go handleMessages(conn, p)
		// fmt.Println(err)
		// os.Exit(1)
	}

	if err := p.Start(); err != nil {
		fmt.Printf("Alas, there's been an error: %v", err)
		os.Exit(1)
	}
}
