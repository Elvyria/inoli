package icons

import (
	"fmt"
)

func battery(level byte) (rune, error) {
	switch {
		case level < 5:   return '', nil
		case level < 35:  return '', nil
		case level < 65:  return '', nil
		case level < 90:  return '', nil
		case level < 101: return '', nil
		default:          return ' ', fmt.Errorf("expected number in range of 0..100, got %v", level)
	}
}
