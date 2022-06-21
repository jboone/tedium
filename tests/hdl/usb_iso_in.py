
from luna.gateware.test.usb2 import USBDeviceTest
from luna.gateware.test.utils import usb_domain_test_case
from luna.gateware.usb.usb2.device import USBDevice

from tedium.gateware.isochronous import USBIsochronousInEndpointTedium


class USBInTest(USBDeviceTest):
    FRAGMENT_UNDER_TEST = USBDevice
    FRAGMENT_ARGUMENTS = {'handle_clocking': False}

    MAX_PACKET_SIZE = 57

    def initialize_signals(self):

        # Keep our device from resetting.
        # yield self.utmi.line_state.eq(0b01)

        # Have our USB device connected.
        yield self.dut.connect.eq(1)

        # Pretend our PHY is always ready to accept data,
        # so we can move forward quickly.
        # yield self.utmi.tx_ready.eq(1)

    def provision_dut(self, dut):
        ep = self.ep = USBIsochronousInEndpointTedium(
            endpoint_number=1,
            max_packet_size=self.MAX_PACKET_SIZE,
        )
        dut.add_endpoint(ep)

    def sof(self):
        yield from self.provide_packet(0b10100101, 0b00111010, 0b00111101)

    @usb_domain_test_case
    def test_usb_in(self):
        yield self.ep.bytes_in_frame.eq(0)
        yield self.ep.value.eq(0x99)
        yield from self.sof()
        pid, packet = yield from self.in_transfer(endpoint=1)
        pid, packet = yield from self.in_transfer(endpoint=1)

        yield self.ep.bytes_in_frame.eq(self.MAX_PACKET_SIZE)
        yield self.ep.value.eq(0x13)
        yield from self.sof()
        pid, packet = yield from self.in_transfer(endpoint=1)
        pid, packet = yield from self.in_transfer(endpoint=1)

        yield self.ep.bytes_in_frame.eq(self.MAX_PACKET_SIZE * 2)
        yield self.ep.value.eq(0x26)
        yield from self.sof()
        pid, packet = yield from self.in_transfer(endpoint=1)
        pid, packet = yield from self.in_transfer(endpoint=1)

        yield from self.sof()
        # pid, packet = yield from self.in_transfer(3, handshake=USBPacketID.NAK)
