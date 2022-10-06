package main

import (
	"strings"
	"fmt"
	"os"

	"github.com/charmbracelet/bubbles/help"
	"github.com/charmbracelet/bubbles/key"
	"github.com/charmbracelet/lipgloss"
	tea "github.com/charmbracelet/bubbletea"
)

type version struct {
	major       int
	minor       int
	maintenance int
	build       int
}

type keymap struct {
	quit key.Binding
}

type model struct {
	title     string
	battery   int
	heartrate int
	steps     int
	firmware  version

	help      help.Model
	keymap    keymap
}

func (m model) Init() tea.Cmd {
	return nil
}

func (m model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch {
		case key.Matches(msg, m.keymap.quit):
			println("\n")
			return m, tea.Quit
		}
	}

	return m, nil
}

var (
	style = lipgloss.NewStyle().Border(lipgloss.RoundedBorder()).Padding(0, 10, 0, 2)

	battery = lipgloss.NewStyle().SetString(" ").String()
	heart = lipgloss.NewStyle().SetString("♥ ").Foreground(lipgloss.Color("#FF4D94")).String()
	human = lipgloss.NewStyle().SetString(" ").String()
)

func (m model) helpView() string {
	return "\n" + m.help.ShortHelpView([]key.Binding{
		m.keymap.quit,
	})
}

func (m model) View() string {
	s := strings.Builder{}

	battery := fmt.Sprintf("%s: %d%%\n", battery, m.battery)
	heartrate := fmt.Sprintf("%s: %d BPM\n", heart, m.heartrate)
	steps := fmt.Sprintf("%s: %d", human, m.steps)

	//s.WriteString(title)
	s.WriteString(battery)
	s.WriteString(heartrate)
	s.WriteString(steps)
	//s.WriteString(firmware)

	return style.Render(s.String()) + m.helpView()
}

func main() {
	model := model {
		title:     "MI1S",
		battery:   78,
		heartrate: 110,
		steps:     1029,

		help:   help.NewModel(),
		keymap: keymap {
			quit: key.NewBinding (
				key.WithKeys("ctrl+c", "q"),
				key.WithHelp("q:", "Quit"),
			),
		},
	}

	p := tea.NewProgram(model)

	if err := p.Start(); err != nil {
		fmt.Printf("Alas, there's been an error: %v", err)
		os.Exit(1)
	}
}
