from enum import IntEnum

from usb_protocol.types import USBTransferType, USBSynchronizationType, USBUsageType, USBDirection
from usb_protocol.emitters import DeviceDescriptorCollection

class Descriptors:

    VENDOR_ID  = 0x16d0
    PRODUCT_ID = 0x0f3b

    class AlternateSetting(IntEnum):
        Idle = 0
        Active = 1

    class InterfaceNumber(IntEnum):
        FrameStream = 0
        Interrupt = 1

    class EndpointNumber(IntEnum):
        FrameStream = 1
        Interrupt = 2

    FRAME_BYTES_MAX = 512

    FRAME_STREAM_ADDITIONAL_PACKETS_PER_INTERVAL = 0
    FRAME_STREAM_INTERVAL = 1   # 2^(bInterval-1) microframes = every 1 microframe.

    INTERRUPT_BYTES_MAX = 256
    INTERRUPT_INTERVAL = 4      # 2^(bInterval-1) microframes = every 8 microframes.

    def create_descriptors(self):

        descriptors = DeviceDescriptorCollection()

        with descriptors.DeviceDescriptor() as d:
            d.idVendor  = self.VENDOR_ID
            d.idProduct = self.PRODUCT_ID

            d.iManufacturer = "ShareBrained"
            d.iProduct      = "Tedium X8"
            d.bcdDevice = 1.01

            d.bNumConfigurations = 1

        with descriptors.ConfigurationDescriptor() as c:

            c.bmAttributes = 0xC0
            c.bMaxPower = 50

            with c.InterfaceDescriptor() as i:
                i.bInterfaceNumber = self.InterfaceNumber.FrameStream
                i.bAlternateSetting = self.AlternateSetting.Idle

            with c.InterfaceDescriptor() as i:
                i.bInterfaceNumber = self.InterfaceNumber.FrameStream
                i.bAlternateSetting = self.AlternateSetting.Active

                with i.EndpointDescriptor() as e:
                    e.bEndpointAddress = USBDirection.IN.to_endpoint_address(self.EndpointNumber.FrameStream)
                    e.wMaxPacketSize   = (self.FRAME_STREAM_ADDITIONAL_PACKETS_PER_INTERVAL << 11) | self.FRAME_BYTES_MAX
                    e.bmAttributes     = (USBUsageType.DATA << 4) | (USBSynchronizationType.NONE << 2) | USBTransferType.ISOCHRONOUS
                    e.bInterval        = self.FRAME_STREAM_INTERVAL

                with i.EndpointDescriptor() as e:
                    e.bEndpointAddress = USBDirection.OUT.to_endpoint_address(self.EndpointNumber.FrameStream)
                    e.wMaxPacketSize   = (self.FRAME_STREAM_ADDITIONAL_PACKETS_PER_INTERVAL << 11) | self.FRAME_BYTES_MAX
                    e.bmAttributes     = (USBUsageType.DATA << 4) | (USBSynchronizationType.NONE << 2) | USBTransferType.ISOCHRONOUS
                    e.bInterval        = self.FRAME_STREAM_INTERVAL

            with c.InterfaceDescriptor() as i:
                i.bInterfaceNumber = self.InterfaceNumber.Interrupt

                # TODO: Interrupt endpoint should move to alternate setting 1?

                with i.EndpointDescriptor() as e:
                    e.bEndpointAddress = USBDirection.IN.to_endpoint_address(self.EndpointNumber.Interrupt)
                    e.wMaxPacketSize   = self.INTERRUPT_BYTES_MAX
                    e.bmAttributes     = USBTransferType.INTERRUPT

                    e.bInterval        = self.INTERRUPT_INTERVAL

        return descriptors
