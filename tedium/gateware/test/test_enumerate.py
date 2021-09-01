#!/usr/bin/env python3

#
# This file is part of Tedium.
#
# Copyright (C) 2020 Jared Boone <jared@sharebrained.com>
# SPDX-License-Identifier: BSD-3-Clause

import logging
import unittest

from usb_protocol.types             import USBDirection, USBTransferType, USBUsageType, USBSynchronizationType, DescriptorTypes
from usb_protocol.emitters     import DeviceDescriptorCollection

from luna.usb2               import USBDevice

from luna.gateware.usb.usb2 import USBPacketID

from luna.gateware.test                   import usb_domain_test_case
from luna.gateware.test.usb2              import USBDeviceTest

# from framer_common import *

class LongConfigDescriptorTest(USBDeviceTest):
    """ :meta private: """

    FRAGMENT_UNDER_TEST = USBDevice
    FRAGMENT_ARGUMENTS = {'handle_clocking': False}

    def traces_of_interest(self):
        return (
            self.utmi.tx_data,
            self.utmi.tx_valid,
            self.utmi.rx_data,
            self.utmi.rx_valid,
        )

    def initialize_signals(self):

        # Keep our device from resetting.
        yield self.utmi.line_state.eq(0b01)

        # Have our USB device connected.
        yield self.dut.connect.eq(1)

        # Pretend our PHY is always ready to accept data,
        # so we can move forward quickly.
        yield self.utmi.tx_ready.eq(1)


    def provision_dut(self, dut):
        self.descriptors = descriptors = DeviceDescriptorCollection()

        with descriptors.DeviceDescriptor() as d:
            d.idVendor  = 0x16d0
            d.idProduct = 0x0f3b
            d.iManufacturer = "ShareBrained"
            d.iProduct      = "Tedium X8"
            d.iSerialNumber = "deadbeef"
            d.bNumConfigurations = 1

        with descriptors.ConfigurationDescriptor() as c:

            with c.InterfaceDescriptor() as i:
                i.bInterfaceNumber = 0

                with i.EndpointDescriptor() as e:
                    e.bEndpointAddress = USBDirection.IN.to_endpoint_address(1)
                    e.wMaxPacketSize   = 64
                    e.bmAttributes     = USBTransferType.INTERRUPT
                    e.bInterval        = 4

            with c.InterfaceDescriptor() as i:
                i.bInterfaceNumber = 1
                i.bAlternateSetting = 0

            with c.InterfaceDescriptor() as i:
                i.bInterfaceNumber = 1
                i.bAlternateSetting = 1

                with i.EndpointDescriptor() as e:
                    e.bEndpointAddress = USBDirection.IN.to_endpoint_address(2)
                    e.wMaxPacketSize   = (1 << 11) | 24
                    e.bmAttributes     = USBTransferType.ISOCHRONOUS
                    e.bInterval        = 1

            with c.InterfaceDescriptor() as i:
                i.bInterfaceNumber = 2
                i.bAlternateSetting = 0

            with c.InterfaceDescriptor() as i:
                i.bInterfaceNumber = 2
                i.bAlternateSetting = 1

                with i.EndpointDescriptor() as e:
                    e.bEndpointAddress = USBDirection.OUT.to_endpoint_address(3)
                    e.wMaxPacketSize   = (1 << 11) | 24
                    e.bmAttributes     = USBTransferType.ISOCHRONOUS
                    e.bInterval        = 1

            # TODO: Force the number of interfaces to "2", as two of the interfaces have multiple
            # alternate settings, and the USB protocol library doesn't seem to care about this.
            from usb_protocol.types.descriptors import StandardDescriptorNumbers
            assert(c._type_counts[StandardDescriptorNumbers.INTERFACE]) == 5
            c._type_counts[StandardDescriptorNumbers.INTERFACE] = 3
            assert(c._type_counts[StandardDescriptorNumbers.INTERFACE]) == 3

        dut.add_standard_control_endpoint(descriptors)

    @usb_domain_test_case
    def test_enumeration(self):
        # Reference enumeration process (quirks merged from Linux, macOS, and Windows):
        # - Read 8 bytes of device descriptor.
        # - Read 64 bytes of device descriptor.
        # - Set address.
        # - Read exact device descriptor length.
        # - Read device qualifier descriptor, three times.
        # - Read config descriptor (without subordinates).
        # - Read language descriptor.
        # - Read Windows extended descriptors. [optional]
        # - Read string descriptors from device descriptor (wIndex=language id).
        # - Set configuration.
        # - Read back configuration number and validate.


        # Read 8 bytes of our device descriptor.
        handshake, data = yield from self.get_descriptor(DescriptorTypes.DEVICE, length=8)
        self.assertEqual(handshake, USBPacketID.ACK)
        self.assertEqual(bytes(data), self.descriptors.get_descriptor_bytes(DescriptorTypes.DEVICE)[0:8])

        # Read 64 bytes of our device descriptor, no matter its length.
        handshake, data = yield from self.get_descriptor(DescriptorTypes.DEVICE, length=64)
        self.assertEqual(handshake, USBPacketID.ACK)
        self.assertEqual(bytes(data), self.descriptors.get_descriptor_bytes(DescriptorTypes.DEVICE))

        # Send a nonsense request, and validate that it's stalled.
        handshake, data = yield from self.control_request_in(0x80, 30, length=10)
        self.assertEqual(handshake, USBPacketID.STALL)

        # Send a set-address request; we'll apply an arbitrary address 0x31.
        yield from self.set_address(0x31)
        self.assertEqual(self.address, 0x31)

        # Read our device descriptor.
        handshake, data = yield from self.get_descriptor(DescriptorTypes.DEVICE, length=18)
        self.assertEqual(handshake, USBPacketID.ACK)
        self.assertEqual(bytes(data), self.descriptors.get_descriptor_bytes(DescriptorTypes.DEVICE))

        # Read our device qualifier descriptor.
        for _ in range(3):
            handshake, data = yield from self.get_descriptor(DescriptorTypes.DEVICE_QUALIFIER, length=10)
            self.assertEqual(handshake, USBPacketID.STALL)

        # Read our configuration descriptor (no subordinates).
        handshake, data = yield from self.get_descriptor(DescriptorTypes.CONFIGURATION, length=9)
        self.assertEqual(handshake, USBPacketID.ACK)
        self.assertEqual(bytes(data), self.descriptors.get_descriptor_bytes(DescriptorTypes.CONFIGURATION)[0:9])

        # Read our configuration descriptor (with subordinates).
        try_config_length = 64
        g = self.get_descriptor(DescriptorTypes.CONFIGURATION, length=try_config_length)
        handshake, data = yield from g
        self.assertEqual(handshake, USBPacketID.ACK)
        self.assertEqual(bytes(data), self.descriptors.get_descriptor_bytes(DescriptorTypes.CONFIGURATION)[:64])

        # Read our string descriptors.
        for i in range(4):
            handshake, data = yield from self.get_descriptor(DescriptorTypes.STRING, index=i, length=255)
            self.assertEqual(handshake, USBPacketID.ACK)
            self.assertEqual(bytes(data), self.descriptors.get_descriptor_bytes(DescriptorTypes.STRING, index=i))

        # Set our configuration...
        status_pid = yield from self.set_configuration(1)
        self.assertEqual(status_pid, USBPacketID.DATA1)

        # ... and ensure it's applied.
        handshake, configuration = yield from self.get_configuration()
        self.assertEqual(handshake, USBPacketID.ACK)
        self.assertEqual(configuration, [1], "device did not accept configuration!")

if __name__ == "__main__":
    unittest.main()
