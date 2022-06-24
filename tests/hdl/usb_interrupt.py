
from luna.gateware.test.usb2 import USBDeviceTest
from luna.gateware.test.utils import usb_domain_test_case
from luna.gateware.usb.usb2.device import USBDevice

from tedium.gateware.usb.status import USBSignalInEndpointTedium

from usb_protocol.types import USBPacketID

class USBInterruptTest(USBDeviceTest):
    FRAGMENT_UNDER_TEST = USBDevice
    FRAGMENT_ARGUMENTS = {'handle_clocking': False}

    ENDPOINT_NUMBER = 3

    def initialize_signals(self):

        # Keep our device from resetting.
        # yield self.utmi.line_state.eq(0b01)

        # Have our USB device connected.
        yield self.dut.connect.eq(1)

        # Pretend our PHY is always ready to accept data,
        # so we can move forward quickly.
        # yield self.utmi.tx_ready.eq(1)

    def provision_dut(self, dut):
        ep = self.ep = USBSignalInEndpointTedium(
            width=32,
            endpoint_number=self.ENDPOINT_NUMBER,
            endianness="big"
        )
        dut.add_endpoint(ep)

    @usb_domain_test_case
    def test_usb_interrupt(self):
        yield self.ep.event_pending.eq(0)
        yield self.ep.signal.eq(0x01234567)
        pid, packet = yield from self.in_transaction(endpoint=self.ENDPOINT_NUMBER, data_pid=None, handshake=USBPacketID.ACK)

        yield self.ep.event_pending.eq(1)
        yield self.ep.signal.eq(0x234269ff)
        pid, packet = yield from self.in_transaction(endpoint=self.ENDPOINT_NUMBER, data_pid=None, handshake=USBPacketID.ACK)
