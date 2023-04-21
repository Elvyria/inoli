package ipc

import (
	"net"

	"inoli-tui/ipc/command"
)

func RequestName(conn net.Conn) error {
	return command.WriteCommand(conn, command.Name, command.Get, nil)
}

func RequestBattery(conn net.Conn) error {
	return command.WriteCommand(conn, command.Battery, command.Get, nil)
}

func RequestSteps(conn net.Conn) error {
	return command.WriteCommand(conn, command.Steps, command.Get, nil)
}

func RequestHeartrate(conn net.Conn) error {
	return command.WriteCommand(conn, command.Heartrate, command.Get, nil)
}

func RequestHeartrateContinuous(conn net.Conn) error {
	return command.WriteCommand(conn, command.HeartrateContinuous, command.Set, []byte { 1 })
}
