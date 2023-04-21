package command

import (
	"fmt"
	"net"
)

type Command byte

const (
	Battery             Command = 83
	Heartrate           Command = 139
	HeartrateContinuous Command = 173
	Name                Command = 244
	Steps               Command = 80
)

type Action byte

const (
	Get Action = 0
	Set Action = 1
)

func WriteCommand(conn net.Conn, cmd Command, action Action, payload []byte) error {
	var header = []byte { 'C', 'M', 'D', byte(cmd), byte(action) }
	var buf = append(header, payload...)

	var n, err = conn.Write(buf)

	if err == nil && n != len(buf) {
		return fmt.Errorf("conn.Write: couldn't send all bytes at once, expected %v, sent %v", len(buf), n)
	}

	return err
}
